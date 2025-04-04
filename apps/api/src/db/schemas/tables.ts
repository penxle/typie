import { eq, sql } from 'drizzle-orm';
import { boolean, index, integer, jsonb, pgTable, text, unique, uniqueIndex } from 'drizzle-orm/pg-core';
import { TableCode } from './codes';
import * as E from './enums';
import { createDbId } from './id';
import { bytea, datetime } from './types';
import type { JSONContent } from '@tiptap/core';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import type { PlanRules } from './json';

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
    type: E._EntityType('type').notNull(),
    order: bytea('order').notNull(),
    state: E._EntityState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex()
      .on(t.slug)
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

export const PaymentInvoices = pgTable('payment_invoices', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PAYMENT_INVOICES)),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  state: E._PaymentInvoiceState('state').notNull().default('UNPAID'),
  amount: integer('amount').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const PaymentMethods = pgTable(
  'payment_methods',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PAYMENT_METHODS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    billingKey: text('billing_key').notNull(),
    state: E._PaymentMethodState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex()
      .on(t.userId)
      .where(sql`${t.state} = 'ACTIVE'`),
  ],
);

export const PaymentRecords = pgTable('payment_records', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PAYMENT_RECORDS)),
  invoiceId: text('invoice_id')
    .notNull()
    .references(() => PaymentInvoices.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  methodId: text('method_id')
    .notNull()
    .references(() => PaymentMethods.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
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
  rules: jsonb('rules').notNull().$type<PlanRules>(),
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
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const PostContents = pgTable('post_contents', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.POST_CONTENTS)),
  postId: text('post_id')
    .notNull()
    .unique()
    .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  title: text('title'),
  subtitle: text('subtitle'),
  body: jsonb('body').notNull().$type<JSONContent>(),
  text: text('text').notNull(),
  update: bytea('update').notNull(),
  vector: bytea('vector').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const PostContentSnapshots = pgTable(
  'post_content_snapshots',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_CONTENT_SNAPSHOTS)),
    postId: text('post_id')
      .notNull()
      .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    snapshot: bytea('snapshot').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.postId, t.createdAt)],
);

export const PostOptions = pgTable('post_options', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.POST_OPTIONS)),
  postId: text('post_id')
    .notNull()
    .unique()
    .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  visibility: E._PostVisibility('visibility').notNull().default('PRIVATE'),
  password: text('password'),
  allowComments: boolean('allow_comments').notNull().default(true),
  allowReactions: boolean('allow_reactions').notNull().default(true),
  allowCopies: boolean('allow_copies').notNull().default(false),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

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
  nextBillingAt: datetime('next_billing_at').notNull(),
  billingDate: integer('billing_date').notNull(),
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
      .references(() => Users.id),
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
