import { GetObjectTaggingCommand, PutObjectTaggingCommand } from '@aws-sdk/client-s3';
import { faker } from '@faker-js/faker';
import * as argon2 from 'argon2';
import dayjs from 'dayjs';
import { and, eq } from 'drizzle-orm';
import { setCookie } from 'hono/cookie';
import ky from 'ky';
import { nanoid } from 'nanoid';
import { match } from 'ts-pattern';
import { redis } from '@/cache';
import { db, first, firstOrThrow, Images, UserMarketingConsents, Users, UserSessions, UserSingleSignOns } from '@/db';
import { sendEmail } from '@/email';
import { PasswordResetEmail, SignUpEmail } from '@/email/templates';
import { SingleSignOnProvider, UserState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import * as aws from '@/external/aws';
import { google, kakao, naver } from '@/external/sso';
import { generateRandomAvatar, persistBlobAsImage } from '@/utils';
import { createSite } from '@/utils/site';
import { builder } from '../builder';
import type { UserContext } from '@/context';
import type { Transaction } from '@/db';

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  loginWithEmail: t.fieldWithInput({
    type: 'Boolean',
    input: {
      email: t.input.string(),
      password: t.input.string(),
    },
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

      if (!(await argon2.verify(user.password, input.password))) {
        throw new TypieError({ code: 'invalid_credentials' });
      }

      await createSession(ctx, user.id);

      return true;
    },
  }),

  sendSignUpEmail: t.fieldWithInput({
    type: 'Boolean',
    input: {
      email: t.input.string(),
      password: t.input.string(),
      name: t.input.string(),
      state: t.input.string(),
      marketingAgreed: t.input.boolean(),
    },
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
          password: await argon2.hash(input.password),
          name: input.name,
          state: input.state,
          marketingAgreed: input.marketingAgreed,
        }),
      );

      await sendEmail({
        recipient: input.email,
        subject: '[타이피] 이메일 주소를 인증해 주세요',
        body: SignUpEmail({
          verificationUrl: `${env.AUTH_URL}/email?code=${code}`,
        }),
      });

      return true;
    },
  }),

  authorizeSignUpEmail: t.fieldWithInput({
    type: 'String',
    input: { code: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const data = await redis.get(`auth:email:${input.code}`);
      if (!data) {
        throw new TypieError({ code: 'invalid_code' });
      }

      const { email, password, name, state, marketingAgreed } = JSON.parse(data);

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

        if (marketingAgreed) {
          await tx.insert(UserMarketingConsents).values({
            userId: user.id,
          });
        }

        return user;
      });

      await redis.del(`auth:email:${input.code}`);

      await createSession(ctx, user.id);

      return state;
    },
  }),

  generateSingleSignOnAuthorizationUrl: t.fieldWithInput({
    type: 'String',
    input: {
      provider: t.input.field({ type: SingleSignOnProvider }),
      email: t.input.string({ required: false }),
      state: t.input.string(),
    },
    resolve: async (_, { input }) => {
      return match(input.provider)
        .with(SingleSignOnProvider.GOOGLE, () => google.generateAuthorizationUrl(input.state, input.email))
        .with(SingleSignOnProvider.NAVER, () => naver.generateAuthorizationUrl(input.state))
        .with(SingleSignOnProvider.KAKAO, () => kakao.generateAuthorizationUrl(input.state))
        .exhaustive();
    },
  }),

  authorizeSingleSignOn: t.fieldWithInput({
    type: 'String',
    input: {
      provider: t.input.field({ type: SingleSignOnProvider }),
      params: t.input.field({ type: 'JSON' }),
    },
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
        await createSession(ctx, sso.userId);

        return input.params.state;
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

        await createSession(ctx, existingUser.id);

        return input.params.state;
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

      await createSession(ctx, user.id);

      return input.params.state;
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
          resetUrl: `${env.AUTH_URL}/reset-password?code=${code}`,
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
        .set({ password: await argon2.hash(input.password) })
        .where(eq(Users.email, email));

      await redis.del(`auth:reset-password:${input.code}`);

      return true;
    },
  }),
}));

/*
 * * Utils
 */

const createSession = async (ctx: UserContext, userId: string) => {
  const token = nanoid(64);
  const expiresAt = dayjs().add(1, 'year');

  await db.insert(UserSessions).values({
    userId,
    token,
    expiresAt,
  });

  setCookie(ctx.c, 'typie-st', token, {
    path: '/',
    httpOnly: true,
    secure: true,
    sameSite: 'lax',
    expires: expiresAt.toDate(),
  });
};

type CreateUserParams = { email: string; name: string; avatarId: string };
const createUser = async (tx: Transaction, { email, name: _name, avatarId }: CreateUserParams) => {
  const name = _name.trim().slice(0, 20);

  const user = await tx.insert(Users).values({ email, name, avatarId }).returning({ id: Users.id }).then(firstOrThrow);

  await createSite({
    userId: user.id,
    name: `${name}의 사이트`,
    slug: [
      faker.word.adjective({ length: { min: 3, max: 5 } }),
      faker.word.noun({ length: { min: 4, max: 6 } }),
      faker.string.numeric({ length: { min: 3, max: 4 } }),
    ].join('-'),
    tx,
  });

  const avatar = await tx
    .update(Images)
    .set({ userId: user.id })
    .where(eq(Images.id, avatarId))
    .returning({ path: Images.path })
    .then(firstOrThrow);

  const tagging = await aws.s3.send(
    new GetObjectTaggingCommand({
      Bucket: 'typie-usercontents',
      Key: `images/${avatar.path}`,
    }),
  );

  const tags: Record<string, string> = {
    ...Object.fromEntries(tagging.TagSet?.map((tag) => [tag.Key, tag.Value]) ?? []),
    UserId: user.id,
  };

  await aws.s3.send(
    new PutObjectTaggingCommand({
      Bucket: 'typie-usercontents',
      Key: `images/${avatar.path}`,
      Tagging: {
        TagSet: Object.entries(tags).map(([key, value]) => ({ Key: key, Value: value })),
      },
    }),
  );

  return user;
};
