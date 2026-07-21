import { PutObjectCommand } from '@aws-sdk/client-s3';
import * as Sentry from '@sentry/node';
import {
  CouponState,
  CreditCodeState,
  EntityState,
  EntityType,
  FontFamilySource,
  FontFamilyState,
  PaymentInvoiceState,
  PlanAvailability,
  SingleSignOnProvider,
  SiteState,
  SubscriptionState,
  UserDevicePlatform,
  UserRole,
  UserState,
} from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { redeemCodeSchema, userSchema } from '@typie/lib/validation';
import argon2 from 'argon2';
import dayjs from 'dayjs';
import { and, asc, desc, eq, gt, gte, inArray, isNotNull, lt, ne, sql, sum } from 'drizzle-orm';
import { nanoid } from 'nanoid';
import qs from 'query-string';
import { redis } from '#/cache.ts';
import {
  CouponRedemptions,
  Coupons,
  CreditCodes,
  db,
  DocumentCharacterCountChanges,
  Entities,
  first,
  firstOrThrow,
  FontFamilies,
  PaymentInvoices,
  Plans,
  ReferralCodes,
  Referrals,
  Sites,
  Subscriptions,
  TableCode,
  UserBillingKeys,
  UserDevices,
  UserInAppPurchases,
  UserMarketingConsents,
  UserPaymentCredits,
  UserPersonalIdentities,
  UserPreferences,
  UserPushNotificationTokens,
  UserRevenues,
  Users,
  UserSessions,
  UserSingleSignOns,
  UserSurveys,
  UserTrials,
  validateDbId,
} from '#/db/index.ts';
import { sendEmail } from '#/email/index.ts';
import { EmailUpdatedEmail, EmailUpdateEmail } from '#/email/templates/index.ts';
import { env, stack } from '#/env.ts';
import * as aws from '#/external/aws.ts';
import * as portone from '#/external/portone.ts';
import { evaluateCouponCondition } from '#/utils/coupon.ts';
import { getDocumentFontFamilies } from '#/utils/document.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { delay } from '#/utils/promise.ts';
import { getUserUsage, getUserUuid } from '#/utils/user.ts';
import { builder } from '../builder.ts';
import {
  CharacterCountChange,
  DocumentFontFamily,
  Entity,
  FontFamily,
  Image,
  isTypeOf,
  IUser,
  PaymentInvoice,
  Post,
  Referral,
  Site,
  Subscription,
  User,
  UserBillingKey,
  UserDevice,
  UserPersonalIdentity,
  UserSingleSignOn,
  UserTrial,
  UserView,
} from '../objects.ts';

/**
 * * Types
 */

IUser.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    avatar: t.expose('avatarId', { type: Image }),
  }),
});

UserView.implement({
  isTypeOf: isTypeOf(TableCode.USERS),
  interfaces: [IUser],
  fields: () => ({}),
});

User.implement({
  isTypeOf: isTypeOf(TableCode.USERS),
  interfaces: [IUser],
  fields: (t) => ({
    email: t.exposeString('email'),
    role: t.expose('role', { type: UserRole }),
    state: t.expose('state', { type: UserState }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),

    trial: t.field({
      type: UserTrial,
      nullable: true,
      resolve: async (self) => {
        return await db.select().from(UserTrials).where(eq(UserTrials.userId, self.id)).then(first);
      },
    }),

    canStartTrial: t.boolean({
      resolve: () => false,
    }),

    uuid: t.string({
      resolve: (self) => getUserUuid(self.id),
    }),

    hasPassword: t.boolean({ resolve: (user) => !!user.password }),

    recentlyViewedEntities: t.field({
      type: [Entity],
      args: { siteId: t.arg.id({ required: false }) },
      resolve: async (self, args) => {
        const entities = await db
          .select()
          .from(Entities)
          .where(
            and(
              eq(Entities.userId, self.id),
              eq(Entities.state, EntityState.ACTIVE),
              isNotNull(Entities.viewedAt),
              ne(Entities.type, EntityType.POST),
              args.siteId ? eq(Entities.siteId, args.siteId) : undefined,
            ),
          )
          .orderBy(desc(Entities.viewedAt))
          .limit(20);

        const entityIds = new Set(entities.map((e) => e.id));
        const folderEntityIds = [
          ...new Set(entities.map((e) => e.parentId).filter((id): id is string => id !== null && !entityIds.has(id))),
        ];

        if (folderEntityIds.length === 0) {
          return entities;
        }

        const folders = await db
          .select()
          .from(Entities)
          .where(and(inArray(Entities.id, folderEntityIds), eq(Entities.state, EntityState.ACTIVE)));

        return [...entities, ...folders];
      },
    }),

    sites: t.field({
      type: [Site],
      resolve: async (self) => {
        return await db
          .select()
          .from(Sites)
          .where(and(eq(Sites.userId, self.id), eq(Sites.state, SiteState.ACTIVE)))
          .orderBy(asc(Sites.createdAt));
      },
    }),

    billingKey: t.field({
      type: UserBillingKey,
      nullable: true,
      resolve: async (self) => {
        return await db.select().from(UserBillingKeys).where(eq(UserBillingKeys.userId, self.id)).then(first);
      },
    }),

    devices: t.field({
      type: [UserDevice],
      resolve: async (self, _args, ctx) => {
        if (ctx.session?.userId !== self.id) {
          throw new TypieError({ code: 'permission_denied' });
        }
        return db
          .select({
            id: UserDevices.id,
            userId: UserDevices.userId,
            identifier: UserDevices.identifier,
            name: UserDevices.name,
            platform: UserDevices.platform,
            lastActiveAt: UserDevices.lastActiveAt,
            lastActiveIp: UserDevices.lastActiveIp,
            createdAt: UserDevices.createdAt,
          })
          .from(UserDevices)
          .innerJoin(UserSessions, eq(UserSessions.deviceId, UserDevices.id))
          .where(eq(UserDevices.userId, self.id))
          .orderBy(desc(UserDevices.lastActiveAt));
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
          .orderBy(desc(eq(Subscriptions.state, SubscriptionState.ACTIVE)), desc(Subscriptions.createdAt))
          .then(first);
      },
    }),

    nextSubscription: t.field({
      type: Subscription,
      nullable: true,
      resolve: async (self) => {
        return await db
          .select()
          .from(Subscriptions)
          .where(and(eq(Subscriptions.userId, self.id), eq(Subscriptions.state, SubscriptionState.WILL_ACTIVATE)))
          .then(first);
      },
    }),

    recentPosts: t.field({
      type: [Post],
      resolve: async () => [],
    }),

    characterCountChanges: t.field({
      type: [CharacterCountChange],
      resolve: async (self) => {
        const startOfTomorrow = dayjs.kst().startOf('day').add(1, 'day');

        const documentDate = sql<string>`DATE(${DocumentCharacterCountChanges.bucket} AT TIME ZONE 'Asia/Seoul')`.mapWith(dayjs.kst);

        return db
          .select({
            date: documentDate,
            additions: sum(DocumentCharacterCountChanges.additions).mapWith(Number),
            deletions: sum(DocumentCharacterCountChanges.deletions).mapWith(Number),
          })
          .from(DocumentCharacterCountChanges)
          .where(
            and(
              eq(DocumentCharacterCountChanges.userId, self.id),
              gte(DocumentCharacterCountChanges.bucket, startOfTomorrow.subtract(365, 'days')),
              lt(DocumentCharacterCountChanges.bucket, startOfTomorrow),
            ),
          )
          .groupBy(documentDate)
          .orderBy(documentDate);
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
        const consent = await db
          .select({ consented: UserMarketingConsents.consented })
          .from(UserMarketingConsents)
          .where(eq(UserMarketingConsents.userId, user.id))
          .then(first);
        return consent?.consented ?? false;
      },
    }),

    marketingConsentAskedAt: t.field({
      type: 'DateTime',
      nullable: true,
      resolve: async (user) => {
        const consent = await db
          .select({ askedAt: UserMarketingConsents.askedAt })
          .from(UserMarketingConsents)
          .where(eq(UserMarketingConsents.userId, user.id))
          .then(first);
        return consent?.askedAt ?? null;
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

    revenue: t.int({
      resolve: async (user) => {
        const revenue = await db
          .select({ amount: UserRevenues.amount })
          .from(UserRevenues)
          .where(eq(UserRevenues.userId, user.id))
          .then(first);

        return revenue?.amount ?? 0;
      },
    }),

    postCount: t.int({
      resolve: async () => 0,
    }),

    documentCount: t.int({
      resolve: async (user) => {
        const result = await db
          .select({ count: sql<number>`count(*)` })
          .from(Entities)
          .where(and(eq(Entities.userId, user.id), eq(Entities.type, EntityType.DOCUMENT), eq(Entities.state, EntityState.ACTIVE)))
          .then(firstOrThrow);
        return Number(result.count);
      },
    }),

    usage: t.field({
      type: t.builder.simpleObject('UserUsage', {
        fields: (t) => ({
          totalCharacterCount: t.int(),
          totalBlobSize: t.field({ type: 'BigInt' }),
        }),
      }),
      resolve: async (self) => {
        const usage = await getUserUsage({ userId: self.id });
        return { ...usage, totalBlobSize: String(usage.totalBlobSize) };
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

        return fontFamilies.toSorted((a, b) => a.familyName.localeCompare(b.familyName));
      },
    }),

    documentFontFamilies: t.field({
      type: [DocumentFontFamily],
      args: {
        sources: t.arg({
          type: [FontFamilySource],
          defaultValue: [FontFamilySource.DEFAULT, FontFamilySource.USER],
        }),
      },
      resolve: async (self, args, ctx) => {
        return await getDocumentFontFamilies(self.id, ctx.session?.userId ?? null, args.sources);
      },
    }),

    paymentInvoices: t.field({
      type: [PaymentInvoice],
      resolve: async (self) => {
        return await db.select().from(PaymentInvoices).where(eq(PaymentInvoices.userId, self.id)).orderBy(desc(PaymentInvoices.createdAt));
      },
    }),

    surveys: t.stringList({
      resolve: async (self) => {
        const results: string[] = [];

        const existingSurveys = await db
          .select({ name: UserSurveys.name })
          .from(UserSurveys)
          .where(
            and(
              eq(UserSurveys.userId, self.id),
              inArray(UserSurveys.name, ['202509_ir', 'trial_expired_modal_shown', 'trial_popup_content_entry_202605']),
            ),
          );

        const shownSurveys = new Set(existingSurveys.map((s) => s.name));
        const trial = await db.select({ id: UserTrials.id }).from(UserTrials).where(eq(UserTrials.userId, self.id)).then(first);
        const subscriptionHistory = await db
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(eq(Subscriptions.userId, self.id))
          .then(first);

        const activeSubscription = await db
          .select({ id: Subscriptions.id })
          .from(Subscriptions)
          .where(
            and(
              eq(Subscriptions.userId, self.id),
              inArray(Subscriptions.state, [SubscriptionState.ACTIVE, SubscriptionState.WILL_EXPIRE, SubscriptionState.IN_GRACE_PERIOD]),
            ),
          )
          .then(first);

        if (!shownSurveys.has('trial_expired_modal_shown') && trial && !activeSubscription) {
          const paidSubscription = await db
            .select({ id: Subscriptions.id })
            .from(Subscriptions)
            .innerJoin(Plans, eq(Subscriptions.planId, Plans.id))
            .where(
              and(
                eq(Subscriptions.userId, self.id),
                inArray(Plans.availability, [PlanAvailability.BILLING_KEY, PlanAvailability.IN_APP_PURCHASE]),
              ),
            )
            .then(first);

          if (!paidSubscription) {
            results.push('trial_expired_modal');
          }
        }

        if (!shownSurveys.has('trial_popup_content_entry_202605') && !trial && !subscriptionHistory) {
          results.push('trial_popup_content_entry_202605');
        }

        if (!shownSurveys.has('202509_ir') && activeSubscription && self.createdAt.isBefore(dayjs().subtract(1, 'weeks'))) {
          results.push('202509_ir');
        }

        return results;
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

UserDevice.implement({
  isTypeOf: isTypeOf(TableCode.USER_DEVICES),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),
    platform: t.expose('platform', { type: UserDevicePlatform }),
    lastActiveAt: t.expose('lastActiveAt', { type: 'DateTime' }),
    lastActiveIp: t.exposeString('lastActiveIp', { nullable: true }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
    isCurrent: t.boolean({
      resolve: (self, _args, ctx) => ctx.session?.deviceId === self.id,
    }),
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

  userView: t.field({
    type: UserView,
    args: { id: t.arg.id() },
    resolve: (_, args) => {
      return args.id;
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
      await db
        .insert(UserMarketingConsents)
        .values({ userId: ctx.session.userId, consented: input.marketingConsent })
        .onConflictDoUpdate({
          target: [UserMarketingConsents.userId],
          set: { consented: input.marketingConsent },
        });

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

      const billingKey = await db.transaction(async (tx) => {
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

        await tx
          .update(Subscriptions)
          .set({ state: SubscriptionState.EXPIRED, expiresAt: sql`LEAST(${Subscriptions.expiresAt}, NOW())` })
          .where(eq(Subscriptions.userId, ctx.session.userId));
        await tx.delete(UserInAppPurchases).where(eq(UserInAppPurchases.userId, ctx.session.userId));

        const billingKey = await tx
          .delete(UserBillingKeys)
          .where(eq(UserBillingKeys.userId, ctx.session.userId))
          .returning({ billingKey: UserBillingKeys.billingKey })
          .then(first);

        await tx.delete(UserPersonalIdentities).where(eq(UserPersonalIdentities.userId, ctx.session.userId));

        await tx.delete(UserSingleSignOns).where(eq(UserSingleSignOns.userId, ctx.session.userId));
        await tx.delete(UserSessions).where(eq(UserSessions.userId, ctx.session.userId));

        await tx.update(Users).set({ state: UserState.DEACTIVATED }).where(eq(Users.id, ctx.session.userId));

        return billingKey;
      });

      if (billingKey) {
        try {
          await portone.deleteBillingKey({ billingKey: billingKey.billingKey });
        } catch (err) {
          Sentry.captureException(err);
        }
      }

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

      const code = input.code.toUpperCase().replaceAll('-', '').replaceAll('O', '0').replaceAll(/[IL]/g, '1');

      return await db.transaction(async (tx) => {
        const creditCode = await tx
          .select({ id: CreditCodes.id, state: CreditCodes.state, amount: CreditCodes.amount })
          .from(CreditCodes)
          .where(and(eq(CreditCodes.code, code), gt(CreditCodes.expiresAt, dayjs())))
          .for('update')
          .then(first);

        if (creditCode) {
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
        }

        const coupon = await tx
          .select()
          .from(Coupons)
          .where(
            and(
              eq(Coupons.code, code),
              eq(Coupons.state, CouponState.ACTIVE),
              lt(Coupons.startsAt, dayjs()),
              gt(Coupons.expiresAt, dayjs()),
            ),
          )
          .for('update')
          .then(first);

        if (coupon) {
          const existingRedemption = await tx
            .select({ id: CouponRedemptions.id })
            .from(CouponRedemptions)
            .where(and(eq(CouponRedemptions.couponId, coupon.id), eq(CouponRedemptions.userId, ctx.session.userId)));

          if (existingRedemption.length > 0) {
            throw new TypieError({ code: 'already_redeemed' });
          }

          const conditionMet = await evaluateCouponCondition(coupon.condition, ctx.session.userId);
          if (!conditionMet) {
            throw new TypieError({ code: 'condition_not_met' });
          }

          await tx.insert(CouponRedemptions).values({
            couponId: coupon.id,
            userId: ctx.session.userId,
            creditAmount: coupon.creditAmount,
          });

          await tx
            .insert(UserPaymentCredits)
            .values({
              userId: ctx.session.userId,
              amount: coupon.creditAmount,
            })
            .onConflictDoUpdate({
              target: [UserPaymentCredits.userId],
              set: {
                amount: sql`${UserPaymentCredits.amount} + ${coupon.creditAmount}`,
              },
            });

          return ctx.session.userId;
        }

        throw new TypieError({ code: 'invalid_code' });
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

  revokeUserDevice: t.withAuth({ session: true }).fieldWithInput({
    type: UserDevice,
    input: { deviceId: t.input.id({ validate: validateDbId(TableCode.USER_DEVICES) }) },
    resolve: async (_, { input }, ctx) => {
      if (input.deviceId === ctx.session.deviceId) {
        throw new TypieError({ code: 'cannot_revoke_current_device' });
      }

      const device = await db
        .select()
        .from(UserDevices)
        .where(and(eq(UserDevices.id, input.deviceId), eq(UserDevices.userId, ctx.session.userId)))
        .then(first);

      if (!device) {
        throw new TypieError({ code: 'permission_denied' });
      }

      await db.delete(UserSessions).where(and(eq(UserSessions.userId, ctx.session.userId), eq(UserSessions.deviceId, device.id)));

      return device;
    },
  }),

  createWsSession: t.withAuth({ session: true }).field({
    type: 'String',
    resolve: async (_, __, ctx) => {
      const token = nanoid(64);

      await redis.setex(
        `user:ws:${token}`,
        60 * 10,
        JSON.stringify({
          sessionId: ctx.session.id,
          userId: ctx.session.userId,
          deviceId: ctx.session.deviceId,
          bootstrapBypassKeyHash: ctx.c.req.header('X-Bootstrap-Bypass'),
        }),
      );

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
      await assertActiveSubscription({ userId: ctx.session.userId });

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

  resetPreferences: t.withAuth({ session: true }).field({
    type: User,
    resolve: async (_, __, ctx) => {
      await assertActiveSubscription({ userId: ctx.session.userId });

      const value = {};

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

  dumpLocalStorage: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: { data: t.input.field({ type: 'JSON' }) },
    resolve: async (_, { input }, ctx) => {
      const timestamp = dayjs().format('YYYY-MM-DD-HHmmss');
      const key = `localstorage-dump/${ctx.session.userId}-${timestamp}.json`;

      const prettyJson = JSON.stringify(input.data, null, 2);

      await aws.s3.send(
        new PutObjectCommand({
          Bucket: 'typie-misc',
          Key: key,
          Body: prettyJson,
          ContentType: 'application/json',
          Tagging: qs.stringify({
            UserId: ctx.session.userId,
            Environment: stack,
          }),
        }),
      );

      return ctx.session.userId;
    },
  }),

  recordSurvey: t.withAuth({ session: true }).fieldWithInput({
    type: User,
    input: {
      name: t.input.string(),
      value: t.input.field({ type: 'JSON' }),
    },
    resolve: async (_, { input }, ctx) => {
      await db
        .insert(UserSurveys)
        .values({
          userId: ctx.session.userId,
          name: input.name,
          value: input.value,
        })
        .onConflictDoUpdate({
          target: [UserSurveys.userId, UserSurveys.name],
          set: { value: input.value },
        });

      return ctx.session.userId;
    },
  }),
}));
