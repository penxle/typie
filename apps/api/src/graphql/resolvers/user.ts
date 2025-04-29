import dayjs from 'dayjs';
import { and, asc, count, desc, eq, getTableColumns, gte, inArray, lt, sql, sum } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import { redis } from '@/cache';
import {
  db,
  Entities,
  first,
  firstOrThrow,
  Notifications,
  PaymentBillingKeys,
  Plans,
  PostCharacterCountChanges,
  Posts,
  Sites,
  TableCode,
  UserPersonalIdentities,
  UserPlans,
  Users,
  UserSessions,
  UserSingleSignOns,
  validateDbId,
} from '@/db';
import { defaultPlanRules } from '@/db/schemas/json';
import { sendEmail } from '@/email';
import { EmailUpdatedEmail, EmailUpdateEmail } from '@/email/templates';
import { EntityState, PaymentBillingKeyState, SingleSignOnProvider, SiteState, UserPlanState, UserState } from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { userSchema } from '@/validation';
import { builder } from '../builder';
import {
  CharacterCountChange,
  Image,
  isTypeOf,
  Notification,
  PaymentBillingKey,
  PlanRule,
  Post,
  Site,
  User,
  UserPersonalIdentity,
  UserPlan,
  UserSingleSignOn,
} from '../objects';

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

    paymentBillingKey: t.field({
      type: PaymentBillingKey,
      nullable: true,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'User.paymentBillingKey',
          nullable: true,
          load: async (ids) => {
            return await db
              .select()
              .from(PaymentBillingKeys)
              .where(and(inArray(PaymentBillingKeys.userId, ids), eq(PaymentBillingKeys.state, PaymentBillingKeyState.ACTIVE)));
          },
          key: (row) => row?.userId,
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

    planRule: t.field({
      type: PlanRule,
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'User.planRule',
          nullable: true,
          load: async (ids) => {
            return await db
              .select({ ...getTableColumns(Plans), userId: UserPlans.userId })
              .from(Plans)
              .innerJoin(UserPlans, eq(Plans.id, UserPlans.planId))
              .where(and(inArray(UserPlans.userId, ids), inArray(UserPlans.state, [UserPlanState.ACTIVE, UserPlanState.CANCELED])));
          },
          key: (row) => row?.userId,
        });

        const plan = await loader.load(self.id);
        return plan?.rules ?? defaultPlanRules;
      },
    }),

    usage: t.expose('id', { type: UserUsage }),

    recentPosts: t.field({
      type: [Post],
      resolve: async (self, _, ctx) => {
        const loader = ctx.loader({
          name: 'User.recentPosts',
          many: true,
          load: async (ids) => {
            return await db
              .select()
              .from(Posts)
              .innerJoin(Entities, eq(Posts.entityId, Entities.id))
              .where(and(inArray(Entities.userId, ids), eq(Entities.state, EntityState.ACTIVE)))
              .orderBy(desc(Posts.updatedAt))
              .limit(5);
          },
          key: ({ entities: { userId } }) => userId,
        });

        const rows = await loader.load(self.id);
        return rows.map((row) => row.posts);
      },
    }),

    characterCountChanges: t.field({
      type: [CharacterCountChange],
      resolve: async (self) => {
        const startOfTomorrow = dayjs.kst().startOf('day').add(1, 'day');

        const date = sql<string>`DATE(${PostCharacterCountChanges.bucket} AT TIME ZONE 'Asia/Seoul')`.mapWith(dayjs.kst);
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
              gte(PostCharacterCountChanges.bucket, startOfTomorrow.subtract(365, 'days')),
              lt(PostCharacterCountChanges.bucket, startOfTomorrow),
            ),
          )
          .groupBy(date)
          .orderBy(asc(date));
      },
    }),

    notifications: t.field({
      type: [Notification],
      resolve: async (user) => {
        return await db.select().from(Notifications).where(eq(Notifications.userId, user.id)).orderBy(desc(Notifications.createdAt));
      },
    }),

    personalIdentity: t.field({
      type: UserPersonalIdentity,
      nullable: true,
      resolve: async (user) => {
        return await db.select().from(UserPersonalIdentities).where(eq(UserPersonalIdentities.userId, user.id)).then(first);
      },
    }),

    singleSignOns: t.field({
      type: [UserSingleSignOn],
      resolve: async (user) => {
        return await db.select().from(UserSingleSignOns).where(eq(UserSingleSignOns.userId, user.id));
      },
    }),
  }),
});

UserPersonalIdentity.implement({
  isTypeOf: isTypeOf(TableCode.USER_PERSONAL_IDENTITIES),
  fields: (t) => ({
    id: t.exposeID('id'),
    birthDate: t.expose('birthDate', { type: 'DateTime' }),
    expiresAt: t.expose('expiresAt', { type: 'DateTime' }),
  }),
});

UserSingleSignOn.implement({
  isTypeOf: isTypeOf(TableCode.USER_SINGLE_SIGN_ONS),
  fields: (t) => ({
    id: t.exposeID('id'),
    provider: t.expose('provider', { type: SingleSignOnProvider }),
    email: t.exposeString('email'),
  }),
});

const UserUsage = builder.objectRef<string>('UserUsage');
UserUsage.implement({
  fields: (t) => ({
    postCount: t.int({
      resolve: async (userId) => {
        return await db
          .select({ count: count() })
          .from(Posts)
          .innerJoin(Entities, eq(Posts.entityId, Entities.id))
          .where(and(eq(Entities.userId, userId), eq(Entities.state, EntityState.ACTIVE)))
          .then((result) => result[0]?.count ?? 0);
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
          verificationUrl: `${env.AUTH_URL}/update-email?code=${code}`,
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
      avatarId: t.input.id({ validate: validateDbId(TableCode.IMAGES) }),
    },
    resolve: async (_, { input }, ctx) => {
      return await db
        .update(Users)
        .set({ name: input.name, avatarId: input.avatarId })
        .where(eq(Users.id, ctx.session.userId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  deleteUser: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({ state: EntityState.DELETED })
          .where(
            inArray(
              Entities.id,
              tx
                .select({ id: Entities.id })
                .from(Entities)
                .innerJoin(Sites, eq(Entities.siteId, Sites.id))
                .where(eq(Sites.userId, ctx.session.userId)),
            ),
          );

        await tx.update(Sites).set({ state: SiteState.DELETED }).where(eq(Sites.userId, ctx.session.userId));

        await tx.delete(UserPlans).where(eq(UserPlans.userId, ctx.session.userId));
        await tx
          .update(PaymentBillingKeys)
          .set({ state: PaymentBillingKeyState.DEACTIVATED })
          .where(eq(PaymentBillingKeys.userId, ctx.session.userId));

        await tx.delete(UserPersonalIdentities).where(eq(UserPersonalIdentities.userId, ctx.session.userId));

        await tx.delete(UserSingleSignOns).where(eq(UserSingleSignOns.userId, ctx.session.userId));
        await tx.delete(UserSessions).where(eq(UserSessions.userId, ctx.session.userId));

        await tx.update(Users).set({ state: UserState.DEACTIVATED }).where(eq(Users.id, ctx.session.userId));
      });

      return true;
    },
  }),

  verifyPersonalIdentity: t.withAuth({ session: true }).fieldWithInput({
    type: UserPersonalIdentity,
    input: { identityVerificationId: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      const resp = await portone.getIdentityVerification({
        identityVerificationId: input.identityVerificationId,
      });

      if (resp.status !== 'succeeded') {
        throw new TypieError({ code: 'identity_verification_failed' });
      }

      const existingIdentityWithSameCi = await db
        .select({ userId: UserPersonalIdentities.userId })
        .from(UserPersonalIdentities)
        .where(eq(UserPersonalIdentities.ci, resp.ci))
        .then(first);

      if (existingIdentityWithSameCi && existingIdentityWithSameCi.userId !== ctx.session.userId) {
        throw new TypieError({ code: 'same_identity_exists' });
      }

      const existingIdentityWithSameUser = await db
        .select({ id: UserPersonalIdentities.id, ci: UserPersonalIdentities.ci })
        .from(UserPersonalIdentities)
        .where(eq(UserPersonalIdentities.userId, ctx.session.userId))
        .then(first);

      if (existingIdentityWithSameUser) {
        if (existingIdentityWithSameUser.ci !== resp.ci) {
          throw new TypieError({ code: 'identity_not_match' });
        }

        return await db
          .update(UserPersonalIdentities)
          .set({
            name: resp.name,
            birthDate: dayjs.kst(resp.birthDate).startOf('day'),
            phoneNumber: resp.phoneNumber,
            ci: resp.ci,
            expiresAt: dayjs.kst().add(1, 'year').startOf('day'),
          })
          .where(eq(UserPersonalIdentities.id, existingIdentityWithSameUser.id))
          .returning()
          .then(firstOrThrow);
      }

      return await db
        .insert(UserPersonalIdentities)
        .values({
          userId: ctx.session.userId,
          name: resp.name,
          birthDate: dayjs.kst(resp.birthDate).startOf('day'),
          gender: resp.gender,
          phoneNumber: resp.phoneNumber,
          ci: resp.ci,
          expiresAt: dayjs.kst().add(1, 'year').startOf('day'),
        })
        .returning()
        .then(firstOrThrow);
    },
  }),

  createWsSession: t.withAuth({ session: true }).field({
    type: 'String',
    resolve: async (_, __, ctx) => {
      const token = nanoid(64);

      await redis.setex(`user:ws:${token}`, 60 * 10, JSON.stringify({ userId: ctx.session.userId }));

      return token;
    },
  }),
}));
