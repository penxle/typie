import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { deleteCookie, setCookie } from 'hono/cookie';
import ky from 'ky';
import { nanoid } from 'nanoid';
import { match } from 'ts-pattern';
import { redis } from '@/cache';
import { db, first, firstOrThrow } from '@/db';
import { Images, Users, UserSessions, UserSingleSignOns } from '@/db/schemas/tables';
import { sendEmail } from '@/email';
import { PasswordResetEmail, SignUpEmail } from '@/email/templates';
import { SingleSignOnProvider, UserState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import { google, kakao, naver } from '@/external/sso';
import { encodeAccessToken, generateRandomAvatar, persistBlobAsImage } from '@/utils';
import { builder } from '../builder';
import { User } from '../objects';
import type { UserContext } from '@/context';
import type { Transaction } from '@/db';

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  loginWithEmail: t.fieldWithInput({
    type: User,
    input: { email: t.input.string(), password: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const email = input.email.toLowerCase();

      const user = await db
        .select({ id: Users.id, password: Users.password })
        .from(Users)
        .where(and(eq(Users.email, email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (!user) {
        throw new TypieError({ code: 'invalid_credentials' });
      }

      if (!user.password) {
        throw new TypieError({ code: 'password_not_set' });
      }

      if (!(await Bun.password.verify(input.password, user.password))) {
        throw new TypieError({ code: 'invalid_credentials' });
      }

      await createSessionAndSetCookie(ctx, user.id);

      return user.id;
    },
  }),

  sendSignUpEmail: t.fieldWithInput({
    type: 'Boolean',
    input: { email: t.input.string(), password: t.input.string(), name: t.input.string() },
    resolve: async (_, { input }) => {
      const email = input.email.toLowerCase();

      const existingUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.email, email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (existingUser) {
        throw new TypieError({ code: 'user_email_exists' });
      }

      const code = nanoid();

      await redis.setex(
        `auth:email:${code}`,
        24 * 60 * 60,
        JSON.stringify({
          email,
          password: await Bun.password.hash(input.password),
          name: input.name,
        }),
      );

      await sendEmail({
        recipient: input.email,
        subject: '[타이피] 이메일 주소를 인증해 주세요',
        body: SignUpEmail({
          verificationUrl: `${env.WEBSITE_URL}/auth/email?code=${code}`,
        }),
      });

      return true;
    },
  }),

  authorizeSignUpEmail: t.fieldWithInput({
    type: User,
    input: { code: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const data = await redis.get(`auth:email:${input.code}`);
      if (!data) {
        throw new TypieError({ code: 'invalid_code' });
      }

      const { email, password, name } = JSON.parse(data);

      const existingUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.email, email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (existingUser) {
        throw new TypieError({ code: 'user_email_exists' });
      }

      const user = await db.transaction(async (tx) => {
        const file = await generateRandomAvatar();
        const avatar = await persistBlobAsImage({ file });

        const user = await createUser(tx, {
          email,
          name,
          avatarId: avatar.id,
        });

        await tx.update(Users).set({ password }).where(eq(Users.id, user.id));

        return user;
      });

      await redis.del(`auth:email:${input.code}`);

      await createSessionAndSetCookie(ctx, user.id);

      return user.id;
    },
  }),

  generateSingleSignOnAuthorizationUrl: t.fieldWithInput({
    type: 'String',
    input: {
      provider: t.input.field({ type: SingleSignOnProvider }),
      email: t.input.field({ type: 'String', required: false }),
    },
    resolve: async (_, { input }) => {
      return match(input.provider)
        .with(SingleSignOnProvider.GOOGLE, () => google.generateAuthorizationUrl(input.email ?? undefined))
        .with(SingleSignOnProvider.NAVER, () => naver.generateAuthorizationUrl())
        .with(SingleSignOnProvider.KAKAO, () => kakao.generateAuthorizationUrl())
        .exhaustive();
    },
  }),

  authorizeSingleSignOn: t.fieldWithInput({
    type: User,
    input: { provider: t.input.field({ type: SingleSignOnProvider }), params: t.input.field({ type: 'JSON' }) },
    resolve: async (_, { input }, ctx) => {
      const externalUser = await match(input.provider)
        .with(SingleSignOnProvider.GOOGLE, () => google.authorizeUser(input.params.code))
        .with(SingleSignOnProvider.NAVER, () => naver.authorizeUser(input.params.code))
        .with(SingleSignOnProvider.KAKAO, () => kakao.authorizeUser(input.params.code))
        .exhaustive();

      const sso = await db
        .select({ userId: UserSingleSignOns.userId })
        .from(UserSingleSignOns)
        .where(and(eq(UserSingleSignOns.provider, externalUser.provider), eq(UserSingleSignOns.principal, externalUser.principal)))
        .then(first);

      if (sso) {
        await createSessionAndSetCookie(ctx, sso.userId);

        return sso.userId;
      }

      const existingUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.email, externalUser.email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (existingUser) {
        await db.insert(UserSingleSignOns).values({
          userId: existingUser.id,
          provider: externalUser.provider,
          principal: externalUser.principal,
          email: externalUser.email,
        });

        await createSessionAndSetCookie(ctx, existingUser.id);

        return existingUser.id;
      }

      const user = await db.transaction(async (tx) => {
        let avatar;
        if (externalUser.avatarUrl) {
          const blob = await ky(externalUser.avatarUrl).blob();
          avatar = await persistBlobAsImage({ file: new File([blob], externalUser.avatarUrl) });
        } else {
          const file = await generateRandomAvatar();
          avatar = await persistBlobAsImage({ file });
        }

        const user = await createUser(tx, {
          email: externalUser.email,
          name: externalUser.name,
          avatarId: avatar.id,
        });

        await tx.insert(UserSingleSignOns).values({
          userId: user.id,
          provider: externalUser.provider,
          principal: externalUser.principal,
          email: externalUser.email,
        });

        return user;
      });

      await createSessionAndSetCookie(ctx, user.id);

      return user.id;
    },
  }),

  sendPasswordResetEmail: t.fieldWithInput({
    type: 'Boolean',
    input: { email: t.input.string() },
    resolve: async (_, { input }) => {
      const email = input.email.toLowerCase();

      const existingUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.email, email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (!existingUser) {
        throw new TypieError({ code: 'user_email_not_found' });
      }

      const code = nanoid();

      await redis.setex(
        `auth:reset-password:${code}`,
        60 * 60,
        JSON.stringify({
          email,
        }),
      );

      await sendEmail({
        recipient: input.email,
        subject: '[타이피] 비밀번호를 재설정해 주세요',
        body: PasswordResetEmail({
          resetUrl: `${env.WEBSITE_URL}/auth/reset-password?code=${code}`,
        }),
      });

      return true;
    },
  }),

  resetPassword: t.fieldWithInput({
    type: 'Boolean',
    input: { code: t.input.string(), password: t.input.string() },
    resolve: async (_, { input }) => {
      const data = await redis.get(`auth:reset-password:${input.code}`);
      if (!data) {
        throw new TypieError({ code: 'invalid_code' });
      }

      const { email } = JSON.parse(data);

      await db
        .update(Users)
        .set({ password: await Bun.password.hash(input.password) })
        .where(eq(Users.email, email));

      await redis.del(`auth:reset-password:${input.code}`);

      return true;
    },
  }),

  logout: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      await db.delete(UserSessions).where(eq(UserSessions.id, ctx.session.id));

      deleteCookie(ctx.c, 'typie-at', {
        path: '/',
        domain: env.COOKIE_DOMAIN,
      });

      return true;
    },
  }),
}));

/*
 * * Utils
 */

const createSessionAndSetCookie = async (ctx: UserContext, userId: string) => {
  const session = await db.insert(UserSessions).values({ userId }).returning({ id: UserSessions.id }).then(firstOrThrow);
  const accessToken = await encodeAccessToken(session.id);

  setCookie(ctx.c, 'typie-at', accessToken, {
    path: '/',
    domain: env.COOKIE_DOMAIN,
    httpOnly: true,
    secure: true,
    sameSite: 'none',
    maxAge: dayjs.duration(1, 'year').asSeconds(),
  });
};

type CreateUserParams = { email: string; name: string; avatarId: string };
const createUser = async (tx: Transaction, { email, name, avatarId }: CreateUserParams) => {
  const user = await tx.insert(Users).values({ email, name, avatarId }).returning({ id: Users.id }).then(firstOrThrow);

  await tx.update(Images).set({ userId: user.id }).where(eq(Images.id, avatarId));

  return user;
};
