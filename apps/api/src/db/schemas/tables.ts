import { eq, sql } from 'drizzle-orm';
import { boolean, index, integer, jsonb, pgTable, text, unique, uniqueIndex } from 'drizzle-orm/pg-core';
import { TableCode } from './codes';
import * as E from './enums';
import { createDbId } from './id';
import { bytea, datetime } from './types';
import type { JSONContent } from '@tiptap/core';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import type { NotificationData, PlanRules } from './json';

export const Comments = pgTable('comments', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.COMMENTS)),
  postId: text('post_id')
    .notNull()
    .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  state: E._CommentState('state').notNull().default('ACTIVE'),
  content: text('content').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const CreditCodes = pgTable('credit_codes', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.CREDIT_CODES)),
  state: E._CreditCodeState('state').notNull().default('AVAILABLE'),
  code: text('code').unique().notNull(),
  amount: integer('amount').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  expiresAt: datetime('expires_at').notNull(),
  redeemedAt: datetime('redeemed_at'),
});

export const Files = pgTable('files', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.FILES)),
  userId: text('user_id').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  name: text('name').notNull(),
  format: text('format').notNull(),
  size: integer('size').notNull(),
  path: text('path').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Folders = pgTable('folders', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.FOLDERS, { length: 'short' })),
  entityId: text('entity_id')
    .notNull()
    .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  name: text('name').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Embeds = pgTable('embeds', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.EMBEDS)),
  userId: text('user_id').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  url: text('url').notNull().unique(),
  type: text('type').notNull(),
  title: text('title'),
  description: text('description'),
  html: text('html'),
  thumbnailUrl: text('thumbnail_url'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Entities = pgTable(
  'entities',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.ENTITIES, { length: 'short' })),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    siteId: text('site_id')
      .notNull()
      .references(() => Sites.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    parentId: text('parent_id').references((): AnyPgColumn => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    slug: text('slug').notNull(),
    permalink: text('permalink').notNull(),
    type: E._EntityType('type').notNull(),
    order: text('order').notNull(),
    depth: integer('depth').notNull().default(0),
    state: E._EntityState('state').notNull().default('ACTIVE'),
    visibility: E._EntityVisibility('visibility').notNull().default('PRIVATE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex()
      .on(t.slug)
      .where(eq(t.state, sql`'ACTIVE'`)),
    uniqueIndex()
      .on(t.permalink)
      .where(eq(t.state, sql`'ACTIVE'`)),
    unique().on(t.siteId, t.parentId, t.order).nullsNotDistinct(),
  ],
);

export const Images = pgTable('images', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.IMAGES)),
  userId: text('user_id').references((): AnyPgColumn => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  name: text('name').notNull(),
  format: text('format').notNull(),
  size: integer('size').notNull(),
  width: integer('width').notNull(),
  height: integer('height').notNull(),
  placeholder: text('placeholder').notNull(),
  path: text('path').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Notifications = pgTable('notifications', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.NOTIFICATIONS)),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  data: jsonb('data').notNull().$type<NotificationData>(),
  state: E._NotificationState('state').notNull().default('UNREAD'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const PaymentBillingKeys = pgTable(
  'payment_billing_keys',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PAYMENT_BILLING_KEYS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    billingKey: text('billing_key').notNull(),
    state: E._PaymentBillingKeyState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex()
      .on(t.userId)
      .where(eq(t.state, sql`'ACTIVE'`)),
  ],
);

export const PaymentInvoices = pgTable('payment_invoices', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PAYMENT_INVOICES)),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  state: E._PaymentInvoiceState('state').notNull(),
  amount: integer('amount').notNull(),
  billingAt: datetime('billing_at').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const PaymentRecords = pgTable('payment_records', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PAYMENT_RECORDS)),
  invoiceId: text('invoice_id')
    .notNull()
    .references(() => PaymentInvoices.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  methodType: E._PaymentMethodType('method_type').notNull(),
  methodId: text('method_id').notNull(),
  state: E._PaymentRecordState('state').notNull(),
  amount: integer('amount').notNull(),
  receiptUrl: text('receipt_url'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Plans = pgTable('plans', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PLANS)),
  name: text('name').notNull(),
  rules: jsonb('rules').notNull().$type<Partial<PlanRules>>(),
  fee: integer('fee').notNull(),
  availability: E._PlanAvailability('availability').notNull().default('PUBLIC'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Posts = pgTable('posts', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.POSTS, { length: 'short' })),
  entityId: text('entity_id')
    .notNull()
    .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  title: text('title'),
  subtitle: text('subtitle'),
  maxWidth: integer('max_width').notNull().default(800),
  coverImageId: text('cover_image_id').references(() => Images.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  password: text('password'),
  contentRating: E._PostContentRating('content_rating').notNull().default('ALL'),
  allowComment: boolean('allow_comment').notNull().default(true),
  allowReaction: boolean('allow_reaction').notNull().default(true),
  protectContent: boolean('protect_content').notNull().default(true),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const PostCharacterCountChanges = pgTable(
  'post_character_count_changes',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_CHARACTER_COUNT_CHANGES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    postId: text('post_id')
      .notNull()
      .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    bucket: datetime('bucket').notNull(),
    additions: integer('additions').notNull().default(0),
    deletions: integer('deletions').notNull().default(0),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.userId, t.postId, t.bucket)],
);

export const PostContents = pgTable('post_contents', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.POST_CONTENTS)),
  postId: text('post_id')
    .notNull()
    .unique()
    .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  body: jsonb('body').notNull().$type<JSONContent>(),
  text: text('text').notNull(),
  characterCount: integer('character_count').notNull().default(0),
  blobSize: integer('blob_size').notNull().default(0),
  update: bytea('update').notNull(),
  vector: bytea('vector').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const PostSnapshots = pgTable(
  'post_snapshots',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_SNAPSHOTS)),
    postId: text('post_id')
      .notNull()
      .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    snapshot: bytea('snapshot').notNull(),
    order: integer('order').notNull().default(0),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.postId, t.createdAt, t.order)],
);

export const PostReactions = pgTable(
  'post_reactions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_REACTIONS)),
    postId: text('post_id')
      .notNull()
      .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    userId: text('user_id').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    deviceId: text('device_id').notNull(),
    emoji: text('emoji').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.postId, t.createdAt)],
);

export const PreorderPayments = pgTable('preorder_payments', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PREORDER_PAYMENTS)),
  email: text('email').notNull(),
  amount: integer('amount').notNull(),
  state: E._PreorderPaymentState('state').notNull().default('PENDING'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const PreorderUsers = pgTable('preorder_users', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PREORDER_USERS)),
  email: text('email').unique().notNull(),
  wish: text('wish'),
  preorderPaymentId: text('preorder_payment_id').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Sites = pgTable(
  'sites',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.SITES, { length: 'short' })),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    slug: text('slug').notNull(),
    name: text('name').notNull(),
    state: E._SiteState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex()
      .on(t.slug)
      .where(eq(t.state, sql`'ACTIVE'`)),
  ],
);

export const Users = pgTable(
  'users',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USERS, { length: 'short' })),
    email: text('email').notNull(),
    password: text('password'),
    name: text('name').notNull(),
    avatarId: text('avatar_id')
      .notNull()
      .references(() => Images.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    state: E._UserState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    index().on(t.email, t.state),
    uniqueIndex()
      .on(t.email)
      .where(eq(t.state, sql`'ACTIVE'`)),
  ],
);

export const UserMarketingConsents = pgTable('user_marketing_consents', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_MARKETING_CONSENTS)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const UserPaymentCredits = pgTable('user_payment_credits', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_PAYMENT_CREDITS)),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  codeId: text('code_id').references(() => CreditCodes.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  initialAmount: integer('initial_amount').notNull(),
  remainingAmount: integer('remaining_amount').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  expiresAt: datetime('expires_at').notNull(),
});

export const UserPaymentCreditTransactions = pgTable('user_payment_credit_transactions', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_PAYMENT_CREDIT_TRANSACTIONS)),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  cause: E._UserPaymentCreditTransactionCause('cause').notNull(),
  amount: integer('amount').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const UserPersonalIdentities = pgTable('user_personal_identities', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_PERSONAL_IDENTITIES)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id),
  name: text('name').notNull(),
  birthDate: datetime('birth_date').notNull(),
  gender: text('gender').notNull(),
  phoneNumber: text('phone_number'),
  ci: text('ci').notNull().unique(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  expiresAt: datetime('expires_at').notNull(),
});

export const UserPlans = pgTable('user_plans', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_PLANS)),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id),
  planId: text('plan_id')
    .notNull()
    .references(() => Plans.id),
  fee: integer('fee').notNull(),
  billingCycle: E._UserPlanBillingCycle('billing_cycle').notNull(),
  state: E._UserPlanState('state').notNull().default('ACTIVE'),
  expiresAt: datetime('expires_at').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const UserSessions = pgTable(
  'user_sessions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_SESSIONS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    token: text('token').notNull().unique(),
    expiresAt: datetime('expires_at').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.userId)],
);

export const UserSingleSignOns = pgTable(
  'user_single_sign_ons',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_SINGLE_SIGN_ONS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id),
    provider: E._SingleSignOnProvider('provider').notNull(),
    principal: text('principal').notNull(),
    email: text('email').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.provider), unique().on(t.provider, t.principal)],
);
