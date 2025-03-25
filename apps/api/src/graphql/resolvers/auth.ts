import { and, eq } from 'drizzle-orm';
import ky from 'ky';
import { match } from 'ts-pattern';
import { db, first, firstOrThrow } from '@/db';
import { Images, Users, UserSessions, UserSingleSignOns } from '@/db/schemas/tables';
import { SingleSignOnProvider, UserState } from '@/enums';
import { GlitterError } from '@/errors';
import { google, kakao, naver } from '@/external/sso';
import { createAccessToken, generateRandomAvatar, persistBlobAsImage } from '@/utils';
import { builder } from '../builder';
import { User } from '../objects';
import type { Transaction } from '@/db';

/**
 * * Types
 */

const UserWithAccessToken = builder.simpleObject('UserWithAccessToken', {
  fields: (t) => ({
    user: t.field({ type: User }),
    accessToken: t.string(),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
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
    type: UserWithAccessToken,
    input: { provider: t.input.field({ type: SingleSignOnProvider }), params: t.input.field({ type: 'JSON' }) },
    resolve: async (_, { input }) => {
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
        return {
          user: sso.userId,
          accessToken: await createSessionAndReturnAccessToken(sso.userId),
        };
      }

      const existingUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.email, externalUser.email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (existingUser) {
        throw new GlitterError({ code: 'user_email_exists' });
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

      return {
        user: user.id,
        accessToken: await createSessionAndReturnAccessToken(user.id),
      };
    },
  }),

  logout: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      await db.delete(UserSessions).where(eq(UserSessions.id, ctx.session.id));

      return true;
    },
  }),
}));

/*
 * * Utils
 */

const createSessionAndReturnAccessToken = async (userId: string) => {
  const session = await db.insert(UserSessions).values({ userId }).returning({ id: UserSessions.id }).then(firstOrThrow);

  return await createAccessToken(session.id);
};

type CreateUserParams = { email: string; name: string; avatarId: string };
const createUser = async (tx: Transaction, { email, name, avatarId }: CreateUserParams) => {
  const user = await tx.insert(Users).values({ email, name, avatarId }).returning({ id: Users.id }).then(firstOrThrow);

  await tx.update(Images).set({ userId: user.id }).where(eq(Images.id, avatarId));

  return user;
};
