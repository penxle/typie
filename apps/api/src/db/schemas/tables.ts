import { eq, sql } from 'drizzle-orm';
import { bigint, boolean, index, integer, jsonb, pgTable, text, unique, uniqueIndex } from 'drizzle-orm/pg-core';
import { TableCode } from './codes';
import * as E from './enums';
import { createDbId } from './id';
import { bytea, datetime } from './types';
import type { JSONContent } from '@tiptap/core';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import type { CanvasShape, PageLayout, PlanRules } from './json';

export const Canvases = pgTable(
  'canvases',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.CANVASES)),
    entityId: text('entity_id')
      .notNull()
      .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    title: text('title'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.entityId)],
);

export const CanvasContents = pgTable('canvas_contents', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.CANVAS_CONTENTS)),
  canvasId: text('canvas_id')
    .notNull()
    .unique()
    .references(() => Canvases.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  shapes: jsonb('shapes').notNull().$type<CanvasShape[]>(),
  update: bytea('update').notNull(),
  vector: bytea('vector').notNull(),
  compactedAt: datetime('compacted_at')
    .notNull()
    .default(sql`now()`),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const CanvasSnapshots = pgTable(
  'canvas_snapshots',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.CANVAS_SNAPSHOTS)),
    canvasId: text('canvas_id')
      .notNull()
      .references(() => Canvases.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    snapshot: bytea('snapshot').notNull(),
    order: integer('order').notNull().default(0),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.canvasId, t.createdAt, t.order)],
);

export const CanvasSnapshotContributors = pgTable(
  'canvas_snapshot_contributors',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.CANVAS_SNAPSHOT_CONTRIBUTORS)),
    snapshotId: text('snapshot_id')
      .notNull()
      .references(() => CanvasSnapshots.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.snapshotId, t.userId)],
);

export const CreditCodes = pgTable('credit_codes', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.CREDIT_CODES)),
  userId: text('user_id').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  code: text('code').unique().notNull(),
  amount: integer('amount').notNull(),
  state: E._CreditCodeState('state').notNull().default('AVAILABLE'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  expiresAt: datetime('expires_at').notNull(),
  usedAt: datetime('used_at'),
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

export const Folders = pgTable(
  'folders',
  {
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
  },
  (t) => [index().on(t.entityId)],
);

export const FontFamilies = pgTable(
  'font_families',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.FONT_FAMILIES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    state: E._FontFamilyState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.name)],
);

export const Fonts = pgTable(
  'fonts',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.FONTS)),
    familyId: text('family_id')
      .notNull()
      .references(() => FontFamilies.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    familyName: text('family_name'),
    fullName: text('full_name'),
    postScriptName: text('post_script_name'),
    weight: integer('weight').notNull(),
    size: integer('size').notNull(),
    path: text('path').notNull(),
    state: E._FontState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.familyId, t.state)],
);

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
    availability: E._EntityAvailability('availability').notNull().default('PRIVATE'),
    viewedAt: datetime('viewed_at'),
    deletedAt: datetime('deleted_at'),
    purgedAt: datetime('purged_at'),
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
    index().on(t.userId, t.state),
    index().on(t.siteId, t.state),
    index().on(t.siteId, t.parentId, t.state),
    index().on(t.parentId, t.state),
    index().on(t.userId, t.viewedAt),
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

export const Notes = pgTable(
  'notes',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.NOTES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    entityId: text('entity_id').references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    content: text('content').notNull(),
    color: text('color').notNull(),
    order: text('order').notNull(),
    state: E._NoteState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    unique().on(t.userId, t.order).nullsNotDistinct(),
    index().on(t.userId, t.state, t.order),
    index().on(t.entityId, t.state, t.order),
  ],
);

export const PaymentInvoices = pgTable(
  'payment_invoices',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PAYMENT_INVOICES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    subscriptionId: text('subscription_id')
      .notNull()
      .references(() => Subscriptions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    amount: integer('amount').notNull(),
    state: E._PaymentInvoiceState('state').notNull(),
    dueAt: datetime('due_at').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.userId, t.state)],
);

export const PaymentRecords = pgTable('payment_records', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PAYMENT_RECORDS)),
  invoiceId: text('invoice_id')
    .notNull()
    .references(() => PaymentInvoices.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  outcome: E._PaymentOutcome('outcome').notNull(),
  billingAmount: integer('billing_amount').notNull(),
  creditAmount: integer('credit_amount').notNull(),
  data: jsonb('data').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Plans = pgTable('plans', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.PLANS)),
  name: text('name').notNull(),
  rule: jsonb('rule').notNull().$type<Partial<PlanRules>>(),
  fee: integer('fee').notNull(),
  interval: E._PlanInterval('interval').notNull(),
  availability: E._PlanAvailability('availability').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Posts = pgTable(
  'posts',
  {
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
    allowReaction: boolean('allow_reaction').notNull().default(true),
    protectContent: boolean('protect_content').notNull().default(true),
    type: E._PostType('type').notNull().default('NORMAL'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.entityId), index().on(t.createdAt), index().on(t.updatedAt)],
);

export const PostAnchors = pgTable(
  'post_anchors',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_ANCHORS)),
    postId: text('post_id')
      .notNull()
      .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    nodeId: text('node_id').notNull(),
    name: text('name'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.postId, t.nodeId)],
);

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
  (t) => [uniqueIndex().on(t.userId, t.postId, t.bucket), index().on(t.userId, t.bucket)],
);

export const PostContents = pgTable(
  'post_contents',
  {
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
    blobSize: bigint('blob_size', { mode: 'number' }).notNull().default(0),
    storedMarks: jsonb('stored_marks').notNull().$type<unknown[]>().default([]),
    layoutMode: E._PostLayoutMode('layout_mode').notNull().default('SCROLL'),
    pageLayout: jsonb('page_layout').$type<PageLayout>(),
    note: text('note').notNull().default(''),
    update: bytea('update').notNull(),
    vector: bytea('vector').notNull(),
    compactedAt: datetime('compacted_at')
      .notNull()
      .default(sql`now()`),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.postId), index().on(t.updatedAt), index().on(t.compactedAt)],
);

export const PostSnapshots = pgTable(
  'post_snapshots',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_SNAPSHOTS)),
    postId: text('post_id')
      .notNull()
      .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    snapshot: bytea('snapshot').notNull(),
    order: integer('order').notNull().default(0),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.postId, t.createdAt, t.order)],
);

export const PostSnapshotContributors = pgTable(
  'post_snapshot_contributors',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.POST_SNAPSHOT_CONTRIBUTORS)),
    snapshotId: text('snapshot_id')
      .notNull()
      .references(() => PostSnapshots.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.snapshotId, t.userId)],
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
  codeId: text('code_id').references(() => CreditCodes.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Referrals = pgTable('referrals', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.REFERRALS)),
  referrerId: text('referrer_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  refereeId: text('referee_id')
    .unique()
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  referrerCompensatedAt: datetime('referrer_compensated_at'),
  refereeCompensatedAt: datetime('referee_compensated_at'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const ReferralCodes = pgTable('referral_codes', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.REFERRAL_CODES)),
  userId: text('user_id')
    .unique()
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  code: text('code').notNull().unique(),
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
    index().on(t.userId, t.state),
  ],
);

export const Subscriptions = pgTable(
  'subscriptions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.SUBSCRIPTIONS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    planId: text('plan_id')
      .notNull()
      .references(() => Plans.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    startsAt: datetime('starts_at').notNull(),
    expiresAt: datetime('expires_at').notNull(),
    state: E._SubscriptionState('state').notNull().default('ACTIVE'),
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
    role: E._UserRole('role').notNull().default('USER'),
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

export const UserBillingKeys = pgTable('user_billing_keys', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_BILLING_KEYS)),
  userId: text('user_id')
    .unique()
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  name: text('name').notNull(),
  billingKey: text('billing_key').unique().notNull(),
  cardNumberHash: text('card_number_hash'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const UserInAppPurchases = pgTable(
  'user_in_app_purchases',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_IN_APP_PURCHASES)),
    userId: text('user_id')
      .notNull()
      .unique()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    store: E._InAppPurchaseStore('store').notNull(),
    identifier: text('identifier').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.store, t.identifier)],
);

export const UserMarketingConsents = pgTable('user_marketing_consents', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_MARKETING_CONSENTS)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
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
    .unique()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
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
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
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

export const UserPreferences = pgTable('user_preferences', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_PREFERENCES)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  value: jsonb('value').notNull().default({}).$type<Record<string, unknown>>(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const UserPushNotificationTokens = pgTable(
  'user_push_notification_tokens',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_PUSH_NOTIFICATION_TOKENS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    token: text('token').notNull().unique(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.userId)],
);

export const UserSessions = pgTable(
  'user_sessions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_SESSIONS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
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
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    provider: E._SingleSignOnProvider('provider').notNull(),
    principal: text('principal').notNull(),
    email: text('email').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.provider), unique().on(t.provider, t.principal)],
);

export const UserSurveys = pgTable(
  'user_surveys',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_SURVEYS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    value: jsonb('value').notNull().default({}).$type<Record<string, unknown>>(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.name)],
);
