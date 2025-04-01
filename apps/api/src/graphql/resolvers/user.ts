import { env } from 'bun';
import { and, eq } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { redis } from '@/cache';
import { db, first, firstOrThrow, Users } from '@/db';
import { sendEmail } from '@/email';
import { EmailChangeEmail, NotifyEmailChangeEmail } from '@/email/templates';
import { UserState } from '@/enums';
import { TypieError } from '@/errors';
import { builder } from '../builder';
import { User } from '../objects';

/**
 * * Types
 */

User.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    email: t.exposeString('email'),
  }),
});

/**
 * * Queries
 */

builder.queryFields((t) => ({
  me: t.field({
    type: User,
    nullable: true,
    resolve: async (_, __, ctx) => {
      return ctx.session?.userId;
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  sendEmailChangeEmail: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: { email: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const email = input.email.toLowerCase();

      const existingUser = await db
        .select({ id: Users.id })
        .from(Users)
        .where(and(eq(Users.email, email), eq(Users.state, UserState.ACTIVE)))
        .then(first);

      if (existingUser) {
        throw new TypieError({ code: 'user_email_exists' });
      }

      const { name } = await db.select({ name: Users.name }).from(Users).where(eq(Users.id, ctx.session.userId)).then(firstOrThrow);

      const code = nanoid();

      await redis.setex(
        `auth:change-email:${code}`,
        24 * 60 * 60,
        JSON.stringify({
          email,
          userId: ctx.session.userId,
        }),
      );

      await sendEmail({
        recipient: input.email,
        subject: '[타이피] 이메일 주소를 인증해 주세요',
        body: EmailChangeEmail({
          name,
          newEmail: email,
          verificationUrl: `${env.WEBSITE_URL}/auth/change-email?code=${code}`,
        }),
      });

      return true;
    },
  }),

  changeEmail: t.fieldWithInput({
    type: 'Boolean',
    input: { code: t.input.string() },
    resolve: async (_, { input }) => {
      const data = await redis.get(`auth:change-email:${input.code}`);
      if (!data) {
        throw new TypieError({ code: 'invalid_code' });
      }

      const { email, userId } = JSON.parse(data) as { email: string; userId: string };

      const { name, oldEmail } = await db
        .select({ name: Users.name, oldEmail: Users.email })
        .from(Users)
        .where(eq(Users.id, userId))
        .then(firstOrThrow);

      await db.update(Users).set({ email }).where(eq(Users.id, userId));

      await redis.del(`auth:change-email:${input.code}`);

      await sendEmail({
        recipient: oldEmail,
        subject: '[타이피] 이메일 주소가 변경되었어요',
        body: NotifyEmailChangeEmail({
          name,
          newEmail: email,
        }),
      });

      return true;
    },
  }),
}));
