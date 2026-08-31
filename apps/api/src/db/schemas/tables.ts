import { eq, sql } from 'drizzle-orm';
import { bigint, boolean, foreignKey, index, integer, jsonb, numeric, pgTable, text, unique, uniqueIndex, uuid } from 'drizzle-orm/pg-core';
import { TableCode } from './codes.ts';
import * as E from './enums.ts';
import { createDbId } from './id.ts';
import { bytea, datetime } from './types.ts';
import type { ConclusionAnchors, Context, ResolvedAnchor, ReviewOutcome, RunUsage } from '@typie/prism';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';
import type { CouponCondition, PlanRules } from './json.ts';

export const DocumentArchivedNodes = pgTable('document_archived_nodes', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.DOCUMENT_ARCHIVED_NODES)),
  content: text('content').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Documents = pgTable(
  'documents',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENTS)),
    entityId: text('entity_id')
      .notNull()
      .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    title: text('title'),
    subtitle: text('subtitle'),
    password: text('password'),
    contentRating: E._DocumentContentRating('content_rating').notNull().default('ALL'),
    allowReaction: boolean('allow_reaction').notNull().default(true),
    protectContent: boolean('protect_content').notNull().default(true),
    locked: boolean('locked').notNull().default(false),
    thumbnailId: text('thumbnail_id').references(() => Images.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    type: E._DocumentType('type').notNull().default('NORMAL'),
    dirtyAt: datetime('dirty_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    index().on(t.entityId),
    index()
      .on(t.dirtyAt)
      .where(sql`dirty_at IS NOT NULL`),
  ],
);

export const DocumentCharacterCountChanges = pgTable(
  'document_character_count_changes',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_CHARACTER_COUNT_CHANGES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    bucket: datetime('bucket').notNull(),
    additions: integer('additions').notNull().default(0),
    deletions: integer('deletions').notNull().default(0),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.userId, t.documentId, t.bucket), index().on(t.userId, t.bucket)],
);

export const Coupons = pgTable('coupons', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.COUPONS)),
  code: text('code').unique().notNull(),
  name: text('name').notNull(),
  description: text('description'),
  creditAmount: integer('credit_amount').notNull(),
  condition: jsonb('condition').$type<CouponCondition>(),
  startsAt: datetime('starts_at').notNull(),
  expiresAt: datetime('expires_at').notNull(),
  state: E._CouponState('state').notNull().default('ACTIVE'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const CouponRedemptions = pgTable(
  'coupon_redemptions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.COUPON_REDEMPTIONS)),
    couponId: text('coupon_id')
      .notNull()
      .references(() => Coupons.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    creditAmount: integer('credit_amount').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.couponId, t.userId), index().on(t.userId), index().on(t.couponId)],
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
  size: bigint('size', { mode: 'number' }).notNull(),
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
    thumbnailId: text('thumbnail_id').references(() => Images.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.entityId)],
);

export const Dividers = pgTable(
  'dividers',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DIVIDERS, { length: 'short' })),
    entityId: text('entity_id')
      .notNull()
      .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
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
    familyName: text('family_name').notNull(),
    state: E._FontFamilyState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.familyName)],
);

export const FontNames = pgTable(
  'font_names',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.FONT_NAMES)),
    fontId: text('font_id')
      .notNull()
      .references(() => Fonts.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    nameId: integer('name_id').notNull(),
    platformId: integer('platform_id').notNull(),
    languageId: integer('language_id').notNull(),
    value: text('value').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.fontId)],
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
    postScriptName: text('post_script_name').notNull(),
    weight: integer('weight').notNull(),
    size: bigint('size', { mode: 'number' }).notNull(),
    path: text('path').notNull(),
    hash: text('hash').notNull().default(''),
    chunks: jsonb('chunks').notNull().default([]),
    state: E._FontState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.familyId, t.state), unique().on(t.familyId, t.postScriptName)],
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
    icon: text('icon').notNull().default('file'),
    iconColor: text('icon_color').notNull().default('gray'),
    viewedAt: datetime('viewed_at'),
    deletedAt: datetime('deleted_at'),
    purgedAt: datetime('purged_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex().on(t.slug),
    uniqueIndex().on(t.permalink),
    unique().on(t.siteId, t.parentId, t.order).nullsNotDistinct(),
    index().on(t.userId, t.state),
    index().on(t.siteId, t.state),
    index().on(t.siteId, t.parentId, t.state),
    index().on(t.parentId, t.state),
    index().on(t.userId, t.viewedAt),
  ],
);

export const EntityGoals = pgTable('entity_goals', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.ENTITY_GOALS)),
  entityId: text('entity_id')
    .notNull()
    .unique()
    .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  targetCharacterCount: integer('target_character_count').notNull(),
  dueAt: datetime('due_at'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const Images = pgTable('images', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.IMAGES)),
  userId: text('user_id').references((): AnyPgColumn => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  name: text('name').notNull(),
  format: text('format').notNull(),
  size: bigint('size', { mode: 'number' }).notNull(),
  width: integer('width').notNull(),
  height: integer('height').notNull(),
  placeholder: text('placeholder').notNull(),
  path: text('path').notNull(),
  originalPath: text('original_path'),
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
    siteId: text('site_id')
      .notNull()
      .references(() => Sites.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    content: text('content').notNull(),
    color: text('color').notNull(),
    order: text('order').notNull(),
    status: E._NoteStatus('status').notNull().default('OPEN'),
    state: E._NoteState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.order).nullsNotDistinct(), index().on(t.userId, t.state, t.order), index().on(t.siteId, t.state)],
);

export const NoteEntities = pgTable(
  'note_entities',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.NOTE_ENTITIES)),
    noteId: text('note_id')
      .notNull()
      .references(() => Notes.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    entityId: text('entity_id')
      .notNull()
      .references(() => Entities.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  },
  (t) => [unique().on(t.noteId, t.entityId), index().on(t.entityId)],
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
    paymentKey: text('payment_key').notNull().unique(),
    servicePeriodStartsAt: datetime('service_period_starts_at').notNull(),
    servicePeriodEndsAt: datetime('service_period_ends_at').notNull(),
    // 마지막 청구 처리 시각 — 성패·PaymentRecords 유무와 무관하게 attemptInvoicePayment 가 스탬프한다.
    // 재시도 크론의 페이싱 신호: 기록은 승인 증거라 PG 미호출 경로에 남지 않아 존재 검사로는 페이스를 잴 수 없다.
    lastAttemptedAt: datetime('last_attempted_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    index().on(t.userId, t.state),
    uniqueIndex('payment_invoices_subscription_service_period_unique').on(t.subscriptionId, t.servicePeriodStartsAt),
    uniqueIndex('payment_invoices_open_subscription_unique')
      .on(t.subscriptionId)
      .where(sql`${t.state} IN ('UPCOMING', 'OVERDUE')`),
    index('payment_invoices_overdue_service_start_index')
      .on(t.servicePeriodStartsAt)
      .where(eq(t.state, sql`'OVERDUE'`)),
  ],
);

export const PaymentRecords = pgTable(
  'payment_records',
  {
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
  },
  (t) => [
    uniqueIndex('payment_records_invoice_success_unique')
      .on(t.invoiceId)
      .where(eq(t.outcome, sql`'SUCCESS'`)),
  ],
);

export const InAppPurchaseRecords = pgTable(
  'in_app_purchase_records',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.IN_APP_PURCHASE_RECORDS)),
    store: E._InAppPurchaseStore('store').notNull(),
    identifier: text('identifier').notNull(),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    productId: text('product_id'),
    state: E._InAppPurchaseRecordState('state').notNull(),
    amount: numeric('amount').notNull(),
    currency: text('currency').notNull(),
    refundedAmount: numeric('refunded_amount'),
    purchasedAt: datetime('purchased_at').notNull(),
    refundedAt: datetime('refunded_at'),
    data: jsonb('data').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.store, t.identifier), index().on(t.userId)],
);

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

export const DocumentReactions = pgTable(
  'document_reactions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_REACTIONS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    userId: text('user_id').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    deviceId: text('device_id').notNull(),
    emoji: text('emoji').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.documentId, t.createdAt)],
);

export const DocumentBundles = pgTable(
  'document_bundles',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_BUNDLES)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    seq: integer('seq').notNull(),
    epoch: integer('epoch').notNull().default(0),
    kind: E._DocumentBundleKind('kind').notNull().default('PUSHED'),
    payload: bytea('payload').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.documentId, t.seq), index().on(t.documentId, t.createdAt)],
);

export const DocumentStates = pgTable('document_states', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.DOCUMENT_STATES)),
  documentId: text('document_id')
    .notNull()
    .unique()
    .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  heads: bytea('heads').notNull(),
  epoch: integer('epoch').notNull().default(0),
  lastBundleSeq: integer('last_bundle_seq').notNull().default(0),
  json: jsonb('json').notNull(),
  text: text('text').notNull(),
  characterCount: integer('character_count').notNull().default(0),
  blobSize: bigint('blob_size', { mode: 'number' }).notNull().default(0),
  projectionDegraded: boolean('projection_degraded').notNull().default(false),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const DocumentHeads = pgTable(
  'document_heads',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_HEADS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    bucket: datetime('bucket').notNull(),
    heads: bytea('heads').notNull(),
    characterCount: integer('character_count').notNull().default(0),
    kind: E._DocumentHeadKind('kind').notNull().default('NORMAL'),
    seq: integer('seq').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.documentId, t.seq), index().on(t.documentId, t.bucket), index().on(t.documentId, t.createdAt)],
);

export const DocumentHeadContributors = pgTable(
  'document_head_contributors',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_HEAD_CONTRIBUTORS)),
    headId: text('head_id')
      .notNull()
      .references(() => DocumentHeads.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    additions: integer('additions'),
    deletions: integer('deletions'),
    excluded: boolean('excluded').notNull().default(false),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    unique().on(t.headId, t.userId),
    index()
      .on(t.userId)
      .where(sql`excluded`),
  ],
);

export const DocumentCommentThreads = pgTable(
  'document_comment_threads',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_COMMENT_THREADS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    selection: jsonb('selection').notNull(),
    state: E._DocumentCommentThreadState('state').notNull().default('ACTIVE'),
    resolvedBy: text('resolved_by').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    resolvedAt: datetime('resolved_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.documentId, t.state)],
);

export const DocumentComments = pgTable(
  'document_comments',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_COMMENTS)),
    threadId: text('thread_id')
      .notNull()
      .references(() => DocumentCommentThreads.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    content: text('content').notNull(),
    state: E._DocumentCommentState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.threadId, t.createdAt)],
);

export const DocumentChangesetsDeadLetter = pgTable('document_changesets_dead_letter', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.DOCUMENT_CHANGESETS_DEAD_LETTER)),
  documentId: text('document_id')
    .notNull()
    .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  payload: bytea('payload').notNull(),
  userId: text('user_id')
    .notNull()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  deviceId: text('device_id')
    .notNull()
    .references((): AnyPgColumn => UserDevices.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  errorMessage: text('error_message').notNull(),
  failedAt: datetime('failed_at')
    .notNull()
    .default(sql`now()`),
});

export const DocumentSweeps = pgTable(
  'document_sweeps',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.DOCUMENT_SWEEPS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    streamSeq: text('stream_seq').notNull(),
    zombieDots: jsonb('zombie_dots').$type<string[]>().notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.documentId, t.streamSeq)],
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

export const PrismSessions = pgTable(
  'prism_sessions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_SESSIONS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    prismAgentId: text('prism_agent_id').notNull().unique(),
    lane: text('lane'),
    openRunSeq: integer('open_run_seq'),
    cursor: integer('cursor').notNull().default(0),
    toolPolicy: E._PrismToolPolicy('tool_policy').notNull().default('STANDARD'),
    title: text('title'),
    awaitingUserAt: datetime('awaiting_user_at'),
    seenAt: datetime('seen_at'),
    archivedAt: datetime('archived_at'),
    deletedAt: datetime('deleted_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.userId, t.updatedAt)],
);

export const PrismToolCalls = pgTable(
  'prism_tool_calls',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_TOOL_CALLS)),
    sessionId: text('session_id')
      .notNull()
      .references(() => PrismSessions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    toolCallId: text('tool_call_id').notNull(),
    tool: text('tool').notNull(),
    resolver: E._PrismToolResolver('resolver').notNull().default('SERVER'),
    result: jsonb('result'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.sessionId, t.toolCallId), index().on(t.sessionId)],
);

export const PrismDocumentEdits = pgTable(
  'prism_document_edits',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_DOCUMENT_EDITS)),
    sessionId: text('session_id')
      .notNull()
      .references(() => PrismSessions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    toolCallId: text('tool_call_id').notNull(),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    beforeHeads: bytea('before_heads').notNull(),
    afterHeads: bytea('after_heads').notNull(),
    checkpointHeads: bytea('checkpoint_heads').notNull(),
    undone: boolean('undone').notNull().default(false),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.sessionId, t.toolCallId), index().on(t.documentId, t.createdAt)],
);

export const PrismWorkflows = pgTable(
  'prism_workflows',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_WORKFLOWS)),
    sessionId: text('session_id')
      .notNull()
      .references(() => PrismSessions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    prismWorkflowId: text('prism_workflow_id').notNull().unique(),
    app: text('app').notNull(),
    name: text('name').notNull(),
    ref: text('ref'),
    state: E._PrismWorkflowState('state').notNull().default('RUNNING'),
    startedAt: datetime('started_at')
      .notNull()
      .default(sql`now()`),
    finishedAt: datetime('finished_at'),
    awaitingUserAt: datetime('awaiting_user_at'),
    usage: jsonb('usage').$type<RunUsage>(),
    error: text('error'),
    cursor: integer('cursor').notNull().default(0),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.sessionId), index().on(t.state)],
);

export const PrismSessionEvents = pgTable(
  'prism_session_events',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_SESSION_EVENTS)),
    sessionId: text('session_id')
      .notNull()
      .references(() => PrismSessions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    seq: integer('seq').notNull(),
    kind: text('kind').notNull(),
    occurredAt: datetime('occurred_at').notNull(),
    loggedAt: datetime('logged_at').notNull(),
    context: jsonb('context').$type<Context | null>(),
    data: jsonb('data').$type<Record<string, unknown>>().notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.sessionId, t.seq)],
);

export const PrismWorkflowEvents = pgTable(
  'prism_workflow_events',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_WORKFLOW_EVENTS)),
    workflowId: text('workflow_id')
      .notNull()
      .references(() => PrismWorkflows.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    seq: integer('seq').notNull(),
    kind: text('kind').notNull(),
    occurredAt: datetime('occurred_at').notNull(),
    loggedAt: datetime('logged_at').notNull(),
    context: jsonb('context').$type<Context | null>(),
    data: jsonb('data').$type<Record<string, unknown>>().notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.workflowId, t.seq)],
);

export const PrismRuns = pgTable(
  'prism_runs',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_RUNS)),
    sessionId: text('session_id')
      .notNull()
      .references(() => PrismSessions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    runSeq: integer('run_seq').notNull(),
    state: E._PrismRunState('state').notNull().default('RUNNING'),
    siteId: text('site_id').references(() => Sites.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    startedAt: datetime('started_at').notNull(),
    finishedAt: datetime('finished_at'),
    reaction: E._PrismReaction('reaction'),
    reactionNote: text('reaction_note'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.sessionId, t.runSeq)],
);

export const PrismReviewDocumentVersions = pgTable(
  'prism_review_document_versions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_REVIEW_DOCUMENT_VERSIONS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    version: integer('version').notNull(),
    title: text('title'),
    subtitle: text('subtitle'),
    content: text('content').notNull(),
    characterCount: integer('character_count').notNull(),
    heads: bytea('heads').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.documentId, t.version)],
);

export const PrismReviewLineages = pgTable(
  'prism_review_lineages',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_REVIEW_LINEAGES)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    tier: E._PrismReviewTier('tier').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.documentId)],
);

export const PrismReviewRounds = pgTable(
  'prism_review_rounds',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_REVIEW_ROUNDS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    round: integer('round').notNull(),
    sessionId: text('session_id').references(() => PrismSessions.id, { onUpdate: 'cascade', onDelete: 'set null' }),
    prismRunSeq: integer('prism_run_seq').notNull(),
    workflowId: text('workflow_id')
      .unique()
      .references(() => PrismWorkflows.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    closedAt: datetime('closed_at'),
    tier: E._PrismReviewTier('tier').notNull(),
    lineageId: text('lineage_id')
      .notNull()
      .references(() => PrismReviewLineages.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    // 이어서일 때 물려받은 회차 — 확인 순간 굳는다(fresh 판별·처분 대조의 기준). 새로 시작은 null
    baseRoundId: text('base_round_id').references((): AnyPgColumn => PrismReviewRounds.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    documentVersionId: text('document_version_id')
      .notNull()
      .references(() => PrismReviewDocumentVersions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    result: jsonb('result').$type<ReviewOutcome>(),
    // 총평(강점·격상) 앵커의 리뷰 시점 캡처 — result와 평행 배열. 사영 전 회차는 null
    conclusionAnchors: jsonb('conclusion_anchors').$type<ConclusionAnchors>(),
    // 사영(좌석·처분·총평 앵커)이 이 회차를 지나간 시각 — 좌석 수와 무관한 "사영됐는가"의 유일한 판정. 앉힐 것이 없던 회차도 찍힌다
    projectedAt: datetime('projected_at'),
    reaction: E._PrismReaction('reaction'),
    reactionNote: text('reaction_note'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.documentId, t.round), index().on(t.sessionId), index().on(t.lineageId)],
);

export const PrismReviewThreads = pgTable(
  'prism_review_threads',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_REVIEW_THREADS)),
    documentId: text('document_id')
      .notNull()
      .references(() => Documents.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    lineageId: text('lineage_id')
      .notNull()
      .references(() => PrismReviewLineages.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    bornRoundId: text('born_round_id')
      .notNull()
      .references(() => PrismReviewRounds.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    issueId: text('issue_id'),
    trait: text('trait').notNull(),
    pass: E._PrismReviewPass('pass').notNull(),
    body: text('body'),
    state: E._PrismReviewThreadState('state').notNull().default('OPEN'),
    stateChangedAt: datetime('state_changed_at'),
    // 해소·철회를 사영한 회차 — 그 회차의 여백이 "정리됨" 갈래로 세운다. 작가가 닫은 스레드는 채우지 않는다
    settledRoundId: text('settled_round_id').references(() => PrismReviewRounds.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    reaction: E._PrismReaction('reaction'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.lineageId), index().on(t.bornRoundId), index().on(t.settledRoundId)],
);

export const PrismReviewThreadSeats = pgTable(
  'prism_review_thread_seats',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_REVIEW_THREAD_SEATS)),
    threadId: text('thread_id')
      .notNull()
      .references(() => PrismReviewThreads.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    roundId: text('round_id')
      .notNull()
      .references(() => PrismReviewRounds.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    issueIndex: integer('issue_index').notNull(),
    anchors: jsonb('anchors').$type<ResolvedAnchor[]>().notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.threadId, t.roundId), unique().on(t.roundId, t.issueIndex), index().on(t.roundId)],
);

export const PrismReviewThreadComments = pgTable(
  'prism_review_thread_comments',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_REVIEW_THREAD_COMMENTS)),
    threadId: text('thread_id')
      .notNull()
      .references(() => PrismReviewThreads.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    author: E._PrismReviewCommentAuthor('author').notNull(),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    body: text('body').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.threadId, t.createdAt)],
);

export const PrismCreditEntries = pgTable(
  'prism_credit_entries',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_CREDIT_ENTRIES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    kind: E._PrismCreditEntryKind('kind').notNull(),
    paidDelta: bigint('paid_delta', { mode: 'number' }).notNull(),
    freeDelta: bigint('free_delta', { mode: 'number' }).notNull(),
    key: text('key'),
    note: text('note'),
    actorId: text('actor_id').references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    expiresAt: datetime('expires_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    unique().on(t.kind, t.key),
    index().on(t.userId),
    index('prism_credit_entries_expires_at_index')
      .on(t.expiresAt)
      .where(sql`${t.expiresAt} is not null`),
  ],
);

export const PrismCreditPurchases = pgTable(
  'prism_credit_purchases',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_CREDIT_PURCHASES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    pack: E._PrismCreditPack('pack').notNull(),
    price: integer('price').notNull(),
    credits: integer('credits').notNull(),
    bonusCredits: integer('bonus_credits').notNull(),
    channel: E._PrismCreditPurchaseChannel('channel').notNull(),
    billingKeyType: E._BillingKeyType('billing_key_type').notNull(),
    paymentKey: text('payment_key').notNull().unique(),
    state: E._PrismCreditPurchaseState('state').notNull(),
    paidAt: datetime('paid_at'),
    data: jsonb('data').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    index().on(t.userId, t.createdAt),
    index('prism_credit_purchases_pending_index')
      .on(t.createdAt)
      .where(sql`${t.state} = 'PENDING'`),
  ],
);

export const PrismCreditRefunds = pgTable(
  'prism_credit_refunds',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.PRISM_CREDIT_REFUNDS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    kind: E._PrismCreditRefundKind('kind').notNull(),
    purchaseId: text('purchase_id').references(() => PrismCreditPurchases.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    amount: integer('amount').notNull(),
    method: E._PrismCreditRefundMethod('method').notNull(),
    state: E._PrismCreditRefundState('state').notNull(),
    actorId: text('actor_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    note: text('note').notNull(),
    data: jsonb('data').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.userId, t.createdAt), index().on(t.purchaseId)],
);

export const Prompts = pgTable('prompts', {
  id: text('id').primaryKey(),
  model: text('model').notNull(),
  effort: text('effort'),
  systemPrompt: text('system_prompt').notNull(),
  toolDescriptions: jsonb('tool_descriptions'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const LlmAnalysisRuns = pgTable(
  'llm_analysis_runs',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.LLM_ANALYSIS_RUNS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    // 클라이언트가 보낸 값이라 존재하지 않는 id면 FK 위반으로 분석 자체가 실패하므로 FK를 걸지 않는다
    documentId: text('document_id'),
    textLength: integer('text_length').notNull(),
    chunkCount: integer('chunk_count').notNull(),
    prefixHash: text('prefix_hash').notNull(),
    fullHash: text('full_hash').notNull(),
    state: E._LlmAnalysisRunState('state').notNull(),
    startedAt: datetime('started_at')
      .notNull()
      .default(sql`now()`),
    endedAt: datetime('ended_at'),
  },
  (t) => [index().on(t.userId, t.startedAt), index().on(t.userId, t.prefixHash)],
);

export const LlmCallUsage = pgTable(
  'llm_call_usage',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.LLM_CALL_USAGE)),
    runId: text('run_id')
      .notNull()
      .references(() => LlmAnalysisRuns.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    phase: text('phase').notNull(),
    model: text('model').notNull(),
    inputTokens: integer('input_tokens'),
    outputTokens: integer('output_tokens'),
    cachedInputTokens: integer('cached_input_tokens'),
    reasoningTokens: integer('reasoning_tokens'),
    totalTokens: integer('total_tokens'),
    inputChars: integer('input_chars').notNull(),
    durationMs: integer('duration_ms').notNull(),
    cacheStatus: text('cache_status'),
    gatewayLogId: text('gateway_log_id'),
    state: E._LlmCallState('state').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.runId)],
);

export const Redirects = pgTable(
  'redirects',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.REDIRECTS)),
    type: E._RedirectType('type').notNull(),
    from: text('from').notNull(),
    to: text('to').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.type, t.from)],
);

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
    logoId: text('logo_id')
      .notNull()
      .references(() => Images.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    slug: text('slug').notNull(),
    name: text('name').notNull(),
    state: E._SiteState('state').notNull().default('ACTIVE'),
    dateDisplay: E._SiteDateDisplay('date_display').notNull().default('UPDATED_AT'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.slug), index().on(t.userId, t.state)],
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
    // 구버전 앱 shim 스냅샷 — 런타임 코드는 읽지도 쓰지도 않는다(백필·마이그레이션 전용)
    expiresAt: datetime('expires_at'),
    currentPeriodStartsAt: datetime('current_period_starts_at').notNull(),
    currentPeriodEndsAt: datetime('current_period_ends_at').notNull(),
    billingAnchorAt: datetime('billing_anchor_at'),
    state: E._SubscriptionState('state').notNull().default('ACTIVE'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    uniqueIndex()
      .on(t.userId)
      .where(eq(t.state, sql`'ACTIVE'`)),
    uniqueIndex('subscriptions_will_activate_user_id_index')
      .on(t.userId)
      .where(eq(t.state, sql`'WILL_ACTIVATE'`)),
    index('subscriptions_will_activate_starts_at_index')
      .on(t.startsAt)
      .where(eq(t.state, sql`'WILL_ACTIVATE'`)),
    index('subscriptions_will_expire_period_ends_index')
      .on(t.currentPeriodEndsAt)
      .where(eq(t.state, sql`'WILL_EXPIRE'`)),
    index('subscriptions_active_period_ends_index')
      .on(t.currentPeriodEndsAt)
      .where(eq(t.state, sql`'ACTIVE'`)),
    index('subscriptions_user_id_state_index').on(t.userId, t.state),
    unique('subscriptions_id_user_id_unique').on(t.id, t.userId),
  ],
);

export const TextReplacements = pgTable('text_replacements', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.TEXT_REPLACEMENTS)),
  match: text('match').notNull(),
  substitute: text('substitute').notNull(),
  regex: boolean('regex').notNull().default(false),
  preset: boolean('preset').notNull().default(false),
  note: text('note'),
  order: text('order'),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const TextReplacementPreferences = pgTable(
  'text_replacement_preferences',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.TEXT_REPLACEMENT_PREFERENCES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    textReplacementId: text('text_replacement_id')
      .notNull()
      .references(() => TextReplacements.id, { onUpdate: 'cascade', onDelete: 'cascade' }),
    state: E._TextReplacementState('state').notNull(),
    order: text('order'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.textReplacementId), unique().on(t.userId, t.order), index().on(t.userId)],
);

export const Users = pgTable(
  'users',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USERS, { length: 'short' })),
    // 스토어에 노출하는 불투명 식별자다(appAccountToken·obfuscatedExternalAccountId). id 와 무관한 난수여야
    // 내부 식별자가 외부로 새지 않는다 — 기존 행만 발급된 트랜잭션과의 호환을 위해 구 파생값(uuid v5)을 유지한다.
    uuid: uuid('uuid')
      .notNull()
      .$defaultFn(() => crypto.randomUUID()),
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
    uniqueIndex().on(t.uuid),
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
  type: E._BillingKeyType('type').notNull().default('CARD'),
  billingKey: text('billing_key').unique().notNull(),
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
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    store: E._InAppPurchaseStore('store').notNull(),
    identifier: text('identifier').notNull(),
    subscriptionId: text('subscription_id'),
    terminatedAt: datetime('terminated_at'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [
    unique().on(t.store, t.identifier),
    unique('user_in_app_purchases_subscription_id_unique').on(t.subscriptionId),
    foreignKey({
      columns: [t.subscriptionId, t.userId],
      foreignColumns: [Subscriptions.id, Subscriptions.userId],
      name: 'user_in_app_purchases_subscription_user_fk',
    }),
  ],
);

export const UserMarketingConsents = pgTable('user_marketing_consents', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_MARKETING_CONSENTS)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  consented: boolean('consented').notNull(),
  askedAt: datetime('asked_at')
    .notNull()
    .default(sql`now()`),
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

export const UserRevenues = pgTable('user_revenues', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_REVENUES)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  amount: integer('amount').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const UserTrials = pgTable('user_trials', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId(TableCode.USER_TRIALS)),
  userId: text('user_id')
    .notNull()
    .unique()
    .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  subscriptionId: text('subscription_id')
    .notNull()
    .references(() => Subscriptions.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  startedAt: datetime('started_at').notNull(),
  expiresAt: datetime('expires_at').notNull(),
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

export const UserGoals = pgTable(
  'user_goals',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_GOALS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    targetCharacterCount: integer('target_character_count'),
    effectiveAt: datetime('effective_at').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [uniqueIndex().on(t.userId, t.effectiveAt)],
);

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

export const UserDevices = pgTable(
  'user_devices',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.USER_DEVICES)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    identifier: text('identifier').notNull(),
    name: text('name').notNull(),
    platform: E._UserDevicePlatform('platform').notNull(),
    lastActiveAt: datetime('last_active_at')
      .notNull()
      .default(sql`now()`),
    lastActiveIp: text('last_active_ip'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.identifier), index().on(t.userId)],
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
    deviceId: text('device_id')
      .notNull()
      .references(() => UserDevices.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    token: text('token').notNull().unique(),
    expiresAt: datetime('expires_at').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.userId), unique().on(t.userId, t.deviceId)],
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

export const Widgets = pgTable(
  'widgets',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId(TableCode.WIDGETS)),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    data: jsonb('data').notNull().default({}).$type<Record<string, unknown>>(),
    order: text('order').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.order), unique().on(t.userId, t.name)],
);
