import dayjs from 'dayjs';
import { and, asc, eq, gte, inArray, lt, sql, sum } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { redis } from '@/cache';
import { db, first, firstOrThrow, PaymentMethods, PostCharacterCountChanges, Sites, TableCode, UserPlans, Users } from '@/db';
import { sendEmail } from '@/email';
import { EmailUpdatedEmail, EmailUpdateEmail } from '@/email/templates';
import { PaymentMethodState, SiteState, UserPlanState, UserState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { userSchema } from '@/validation';
import { builder } from '../builder';
import { CharacterCountChange, Image, isTypeOf, PaymentMethod, Site, User, UserPlan } from '../objects';

/**
 * * Types
 */

User.implement({
  isTypeOf: isTypeOf(TableCode.USERS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    email: t.exposeString('email'),
    avatar: t.expose('avatarId', { type: Image }),

    sites: t.field({
      type: [Site],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'User.sites',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Sites)
              .where(and(inArray(Sites.userId, ids), eq(Sites.state, SiteState.ACTIVE)));
          },
          key: ({ userId }) => userId,
        });

        return await loader.load(self.id);
      },
    }),

    paymentMethod: t.field({
      type: PaymentMethod,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'User.paymentMethod',
          load: async (ids) => {
            return await db
              .select()
              .from(PaymentMethods)
              .where(and(inArray(PaymentMethods.userId, ids), eq(PaymentMethods.state, PaymentMethodState.ACTIVE)));
          },
          key: ({ userId }) => userId,
        });

        return await loader.load(self.id);
      },
    }),

    plan: t.field({
      type: UserPlan,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'User.enrolledPlan',
          nullable: true,
          load: async (ids) => {
            return await db
              .select()
              .from(UserPlans)
              .where(and(inArray(UserPlans.userId, ids), inArray(UserPlans.state, [UserPlanState.ACTIVE, UserPlanState.CANCELED])));
          },
          key: (row) => row?.userId,
        });

        return await loader.load(self.id);
      },
    }),

    characterCountChanges: t.field({
      type: [CharacterCountChange],
      resolve: async (self) => {
        const startOfTomorrow = dayjs.kst().startOf('day').add(1, 'day');

        const date = sql<string>`DATE(${PostCharacterCountChanges.timestamp} AT TIME ZONE 'Asia/Seoul')`.mapWith(dayjs.utc);
        return await db
          .select({
            date,
            additions: sum(PostCharacterCountChanges.additions).mapWith(Number),
            deletions: sum(PostCharacterCountChanges.deletions).mapWith(Number),
          })
          .from(PostCharacterCountChanges)
          .where(
            and(
              eq(PostCharacterCountChanges.userId, self.id),
              gte(PostCharacterCountChanges.timestamp, startOfTomorrow.subtract(365, 'days')),
              lt(PostCharacterCountChanges.timestamp, startOfTomorrow),
            ),
          )
          .groupBy(date)
          .orderBy(asc(date));
      },
    }),
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
  sendEmailUpdateEmail: t.withAuth({ session: true }).fieldWithInput({
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

      const user = await db.select({ name: Users.name }).from(Users).where(eq(Users.id, ctx.session.userId)).then(firstOrThrow);

      const code = nanoid();

      await redis.setex(
        `user:update-email:${code}`,
        24 * 60 * 60,
        JSON.stringify({
          email,
          userId: ctx.session.userId,
        }),
      );

      await sendEmail({
        recipient: input.email,
        subject: '[타이피] 이메일 주소를 인증해 주세요',
        body: EmailUpdateEmail({
          name: user.name,
          email,
          verificationUrl: `${env.WEBSITE_URL}/auth/update-email?code=${code}`,
        }),
      });

      return true;
    },
  }),

  updateEmail: t.fieldWithInput({
    type: 'Boolean',
    input: { code: t.input.string() },
    resolve: async (_, { input }) => {
      const data = await redis.get(`user:update-email:${input.code}`);
      if (!data) {
        throw new TypieError({ code: 'invalid_code' });
      }

      const { email, userId } = JSON.parse(data);

      const user = await db
        .select({ id: Users.id, name: Users.name, email: Users.email })
        .from(Users)
        .where(and(eq(Users.id, userId), eq(Users.state, UserState.ACTIVE)))
        .then(firstOrThrow);

      await db.update(Users).set({ email }).where(eq(Users.id, user.id));

      await redis.del(`user:update-email:${input.code}`);

      await sendEmail({
        recipient: user.email,
        subject: '[타이피] 이메일 주소가 변경되었어요',
        body: EmailUpdatedEmail({
          name: user.name,
          email,
        }),
      });

      return true;
    },
  }),

  updateUser: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: {
      name: t.input.string({ validate: { schema: userSchema.name } }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db.update(Users).set({ name: input.name }).where(eq(Users.id, ctx.session.userId)).returning().then(firstOrThrow);
    },
  }),

  verifyIdentity: t.fieldWithInput({
    type: 'Boolean',
    input: { identityVerificationId: t.input.string() },
    resolve: async (_, { input }) => {
      const resp = await portone.getIdentityVerification({ identityVerificationId: input.identityVerificationId });

      if (resp.status !== 'succeeded') {
        throw new TypieError({ code: 'identity_verification_failed' });
      }

      return true;
    },
  }),
}));
