import { and, eq } from 'drizzle-orm';
import { match } from 'ts-pattern';
import { db, first, firstOrThrow } from '@/db';
import { Users, UserSessions, UserSingleSignOns } from '@/db/schemas/tables';
import { SingleSignOnProvider, UserState } from '@/enums';
import { GlitterError } from '@/errors';
import * as google from '@/external/google';
import { createAccessToken } from '@/utils/access-token';
import { builder } from '../builder';
import { User } from '../objects';

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
        .exhaustive();
    },
  }),

  authorizeSingleSignOn: t.fieldWithInput({
    type: UserWithAccessToken,
    input: { provider: t.input.field({ type: SingleSignOnProvider }), params: t.input.field({ type: 'JSON' }) },
    resolve: async (_, { input }) => {
      const externalUser = await match(input.provider)
        .with(SingleSignOnProvider.GOOGLE, () => google.authorizeUser(input.params.code))
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
          isNewUser: false,
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
        const user = await tx
          .insert(Users)
          .values({ email: externalUser.email, name: externalUser.name })
          .returning({ id: Users.id })
          .then(firstOrThrow);

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
        isNewUser: true,
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
