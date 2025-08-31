import argon2 from 'argon2';
import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, gte, inArray, isNotNull, lt, sql, sum } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import * as uuid from 'uuid';
import { redis } from '@/cache';
import {
  CreditCodes,
  db,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  FontFamilies,
  Notifications,
  PaymentInvoices,
  PostCharacterCountChanges,
  Posts,
  ReferralCodes,
  Referrals,
  Sites,
  Subscriptions,
  TableCode,
  UserBillingKeys,
  UserInAppPurchases,
  UserMarketingConsents,
  UserPaymentCredits,
  UserPersonalIdentities,
  UserPreferences,
  UserPushNotificationTokens,
  Users,
  UserSessions,
  UserSingleSignOns,
  validateDbId,
} from '@/db';
import { sendEmail } from '@/email';
import { EmailUpdatedEmail, EmailUpdateEmail } from '@/email/templates';
import {
  CreditCodeState,
  EntityState,
  FontFamilyState,
  PaymentInvoiceState,
  SingleSignOnProvider,
  SiteState,
  SubscriptionState,
  UserRole,
  UserState,
} from '@/enums';
import { env } from '@/env';
import { TypieError } from '@/errors';
import * as portone from '@/external/portone';
import { delay } from '@/utils/promise';
import { getUserUsage } from '@/utils/user';
import { redeemCodeSchema, userSchema } from '@/validation';
import { builder } from '../builder';
import {
  CharacterCountChange,
  Entity,
  FontFamily,
  Image,
  isTypeOf,
  Notification,
  Post,
  Referral,
  Site,
  Subscription,
  User,
  UserBillingKey,
  UserPersonalIdentity,
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
    role: t.expose('role', { type: UserRole }),
    state: t.expose('state', { type: UserState }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),

    uuid: t.string({
      // just a randomly-picked uuid for namespace
      resolve: (self) => uuid.v5(self.id, '1d394eb5-c61c-4c49-944e-05c9f9435adf'),
    }),

    hasPassword: t.boolean({ resolve: (user) => !!user.password }),

    recentlyViewedEntities: t.field({
      type: [Entity],
      resolve: async (self) => {
        return await db
          .select()
          .from(Entities)
          .where(and(eq(Entities.userId, self.id), eq(Entities.state, EntityState.ACTIVE), isNotNull(Entities.viewedAt)))
          .orderBy(desc(Entities.viewedAt))
          .limit(10);
      },
    }),

    sites: t.field({
      type: [Site],
      resolve: async (self) => {
        return await db
          .select()
          .from(Sites)
          .where(and(eq(Sites.userId, self.id), eq(Sites.state, SiteState.ACTIVE)))
          .orderBy(desc(Sites.createdAt));
      },
    }),

    billingKey: t.field({
      type: UserBillingKey,
      nullable: true,
      resolve: async (self) => {
        return await db.select().from(UserBillingKeys).where(eq(UserBillingKeys.userId, self.id)).then(first);
      },
    }),

    subscription: t.field({
      type: Subscription,
      nullable: true,
      resolve: async (self) => {
        return await db
          .select()
          .from(Subscriptions)
          .where(
            and(
              eq(Subscriptions.userId, self.id),
              inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
            ),
          )
          .then(first);
      },
    }),

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

    marketingConsent: t.field({
      type: 'Boolean',
      resolve: async (user) => {
        return !!(await db
          .select({ id: UserMarketingConsents.id })
          .from(UserMarketingConsents)
          .where(eq(UserMarketingConsents.userId, user.id))
          .then(first));
      },
    }),

    credit: t.int({
      resolve: async (user) => {
        const credit = await db
          .select({ amount: UserPaymentCredits.amount })
          .from(UserPaymentCredits)
          .where(eq(UserPaymentCredits.userId, user.id))
          .then(first);

        return credit?.amount ?? 0;
      },
    }),

    postCount: t.int({
      resolve: async (user) => {
        const result = await db
          .select({ count: sql<number>`count(*)` })
          .from(Posts)
          .innerJoin(Entities, eq(Posts.entityId, Entities.id))
          .where(and(eq(Entities.userId, user.id), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);

        return Number(result.count);
      },
    }),

    totalCharacterCount: t.int({
      resolve: async (user) => {
        const result = await db
          .select({
            total: sql<number>`COALESCE(SUM(${PostCharacterCountChanges.additions}), 0) - COALESCE(SUM(${PostCharacterCountChanges.deletions}), 0)`,
          })
          .from(PostCharacterCountChanges)
          .where(eq(PostCharacterCountChanges.userId, user.id))
          .then(firstOrThrow);
        return Math.max(0, Number(result.total));
      },
    }),

    usage: t.field({
      type: t.builder.simpleObject('UserUsage', {
        fields: (t) => ({
          totalCharacterCount: t.int(),
          totalBlobSize: t.int(),
        }),
      }),
      resolve: async (self) => {
        return await getUserUsage({ userId: self.id });
      },
    }),

    referrals: t.field({
      type: [Referral],
      resolve: async (self) => {
        return await db.select().from(Referrals).where(eq(Referrals.referrerId, self.id)).orderBy(desc(Referrals.createdAt));
      },
    }),

    referral: t.field({
      type: Referral,
      nullable: true,
      resolve: async (self) => {
        return await db.select().from(Referrals).where(eq(Referrals.refereeId, self.id)).then(first);
      },
    }),

    preferences: t.field({
      type: 'JSON',
      resolve: async (self) => {
        const preference = await db
          .select({ value: UserPreferences.value })
          .from(UserPreferences)
          .where(eq(UserPreferences.userId, self.id))
          .then(first);

        return preference?.value ?? {};
      },
    }),

    fontFamilies: t.field({
      type: [FontFamily],
      resolve: async (self) => {
        const fontFamilies = await db
          .select()
          .from(FontFamilies)
          .where(and(eq(FontFamilies.userId, self.id), eq(FontFamilies.state, FontFamilyState.ACTIVE)));

        return fontFamilies.sort((a, b) => a.name.localeCompare(b.name));
      },
    }),
  }),
});

UserBillingKey.implement({
  isTypeOf: isTypeOf(TableCode.USER_BILLING_KEYS),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

UserPersonalIdentity.implement({
  isTypeOf: isTypeOf(TableCode.USER_PERSONAL_IDENTITIES),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    birthDate: t.expose('birthDate', { type: 'DateTime' }),
    gender: t.exposeString('gender'),
    phoneNumber: t.exposeString('phoneNumber', { nullable: true }),
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

Referral.implement({
  isTypeOf: isTypeOf(TableCode.REFERRALS),
  fields: (t) => ({
    id: t.exposeID('id'),
    compensated: t.boolean({ resolve: (self) => !!self.referrerCompensatedAt }),
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

  updateMarketingConsent: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: {
      marketingConsent: t.input.boolean(),
    },
    resolve: async (_, { input }, ctx) => {
      if (input.marketingConsent) {
        await db.insert(UserMarketingConsents).values({ userId: ctx.session.userId }).onConflictDoNothing();
      } else {
        await db.delete(UserMarketingConsents).where(eq(UserMarketingConsents.userId, ctx.session.userId));
      }

      return ctx.session.userId;
    },
  }),

  deleteUser: t.withAuth({ session: true }).field({
    type: 'Boolean',
    resolve: async (_, __, ctx) => {
      const overdueInvoices = await db
        .select({ id: PaymentInvoices.id })
        .from(PaymentInvoices)
        .where(and(eq(PaymentInvoices.userId, ctx.session.userId), eq(PaymentInvoices.state, PaymentInvoiceState.OVERDUE)));

      if (overdueInvoices.length > 0) {
        throw new TypieError({ code: 'overdue_invoices_exist' });
      }

      await db.transaction(async (tx) => {
        await tx
          .update(Entities)
          .set({ state: EntityState.PURGED, purgedAt: dayjs() })
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

        await tx.update(Subscriptions).set({ state: SubscriptionState.EXPIRED }).where(eq(Subscriptions.userId, ctx.session.userId));
        await tx.delete(UserBillingKeys).where(eq(UserBillingKeys.userId, ctx.session.userId));
        await tx.delete(UserInAppPurchases).where(eq(UserInAppPurchases.userId, ctx.session.userId));

        await tx.delete(UserPersonalIdentities).where(eq(UserPersonalIdentities.userId, ctx.session.userId));

        await tx.delete(UserSingleSignOns).where(eq(UserSingleSignOns.userId, ctx.session.userId));
        await tx.delete(UserSessions).where(eq(UserSessions.userId, ctx.session.userId));

        await tx.update(Users).set({ state: UserState.DEACTIVATED }).where(eq(Users.id, ctx.session.userId));
      });

      return true;
    },
  }),

  verifyPersonalIdentity: t.withAuth({ session: true }).fieldWithInput({
    type: User,
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

        await db
          .update(UserPersonalIdentities)
          .set({
            name: resp.name,
            birthDate: dayjs.kst(resp.birthDate).startOf('day'),
            phoneNumber: resp.phoneNumber,
            ci: resp.ci,
            expiresAt: dayjs.kst().add(1, 'year').startOf('day'),
          })
          .where(eq(UserPersonalIdentities.id, existingIdentityWithSameUser.id));
      } else {
        await db.insert(UserPersonalIdentities).values({
          userId: ctx.session.userId,
          name: resp.name,
          birthDate: dayjs.kst(resp.birthDate).startOf('day'),
          gender: resp.gender,
          phoneNumber: resp.phoneNumber,
          ci: resp.ci,
          expiresAt: dayjs.kst().add(1, 'year').startOf('day'),
        });
      }

      return ctx.session.userId;
    },
  }),

  redeemCreditCode: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: { code: t.input.string({ validate: { schema: redeemCodeSchema } }) },
    resolve: async (_, { input }, ctx) => {
      await delay(Math.random() * 2000);

      const code = input.code.toUpperCase().replaceAll('-', '').replaceAll('O', '0').replaceAll('I', '1').replaceAll('L', '1');

      return await db.transaction(async (tx) => {
        const creditCode = await tx
          .select({ id: CreditCodes.id, state: CreditCodes.state, amount: CreditCodes.amount })
          .from(CreditCodes)
          .where(and(eq(CreditCodes.code, code), gt(CreditCodes.expiresAt, dayjs())))
          .for('update')
          .then(firstOrThrowWith(new TypieError({ code: 'invalid_code' })));

        if (creditCode.state === CreditCodeState.USED) {
          throw new TypieError({ code: 'already_redeemed' });
        }

        await tx
          .update(CreditCodes)
          .set({
            userId: ctx.session.userId,
            state: CreditCodeState.USED,
            usedAt: dayjs(),
          })
          .where(eq(CreditCodes.id, creditCode.id));

        await tx
          .insert(UserPaymentCredits)
          .values({
            userId: ctx.session.userId,
            amount: creditCode.amount,
          })
          .onConflictDoUpdate({
            target: [UserPaymentCredits.userId],
            set: {
              amount: sql`${UserPaymentCredits.amount} + ${creditCode.amount}`,
            },
          });

        return ctx.session.userId;
      });
    },
  }),

  registerPushNotificationToken: t.withAuth({ session: true }).fieldWithInput({
    type: 'Boolean',
    input: { token: t.input.string() },
    resolve: async (_, { input }, ctx) => {
      await db
        .insert(UserPushNotificationTokens)
        .values({
          userId: ctx.session.userId,
          token: input.token,
        })
        .onConflictDoUpdate({
          target: [UserPushNotificationTokens.token],
          set: { userId: ctx.session.userId },
        });

      return true;
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

  updatePassword: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: {
      currentPassword: t.input.string({ required: false }),
      newPassword: t.input.string(),
    },
    resolve: async (_, { input }, ctx) => {
      const user = await db.select({ password: Users.password }).from(Users).where(eq(Users.id, ctx.session.userId)).then(firstOrThrow);

      if (user.password) {
        if (!input.currentPassword) {
          throw new TypieError({ code: 'current_password_required' });
        }

        if (!(await argon2.verify(user.password, input.currentPassword))) {
          throw new TypieError({ code: 'invalid_password' });
        }
      }

      return await db
        .update(Users)
        .set({ password: await argon2.hash(input.newPassword) })
        .where(eq(Users.id, ctx.session.userId))
        .returning()
        .then(firstOrThrow);
    },
  }),

  issueReferralUrl: t.withAuth({ session: true }).field({
    type: 'String',
    resolve: async (_, __, ctx) => {
      const host = env.USERSITE_URL.replace('*.', '');

      const existingCode = await db
        .select({ code: ReferralCodes.code })
        .from(ReferralCodes)
        .where(eq(ReferralCodes.userId, ctx.session.userId))
        .then(first);

      if (existingCode) {
        return `${host}/r/${existingCode.code}`;
      }

      const code = nanoid(6);
      await db.insert(ReferralCodes).values({ userId: ctx.session.userId, code });

      return `${host}/r/${code}`;
    },
  }),

  updatePreferences: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: { value: t.input.field({ type: 'JSON' }) },
    resolve: async (_, { input }, ctx) => {
      const preference = await db
        .select({ value: UserPreferences.value })
        .from(UserPreferences)
        .where(eq(UserPreferences.userId, ctx.session.userId))
        .then(first);

      const value = { ...preference?.value, ...input.value };

      await db
        .insert(UserPreferences)
        .values({ userId: ctx.session.userId, value })
        .onConflictDoUpdate({
          target: [UserPreferences.userId],
          set: { value },
        });

      return ctx.session.userId;
    },
  }),
}));
