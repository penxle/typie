// spell-checker:ignoreRegExp /createDbId\('[A-Z]{1,4}'/g

import { eq, sql } from 'drizzle-orm';
import { index, integer, pgTable, text, unique, uniqueIndex } from 'drizzle-orm/pg-core';
import * as E from './enums';
import { createDbId } from './id';
import { bytea, datetime } from './types';
import type { AnyPgColumn } from 'drizzle-orm/pg-core';

export const Files = pgTable('files', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId('FILE')),
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
      .$defaultFn(() => createDbId('F', { length: 'short' })),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    parentId: text('parent_id').references((): AnyPgColumn => Folders.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    name: text('name').notNull(),
    state: E._FolderState('state').notNull().default('ACTIVE'),
    order: bytea('order').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.parentId, t.order).nullsNotDistinct()],
);

export const Embeds = pgTable('embeds', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId('EMBD')),
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

export const Images = pgTable('images', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId('IMG')),
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

export const Posts = pgTable(
  'posts',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId('P', { length: 'short' })),
    userId: text('user_id')
      .notNull()
      .references(() => Users.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    folderId: text('folder_id').references(() => Folders.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
    state: E._PostState('state').notNull().default('ACTIVE'),
    order: bytea('order').notNull(),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [unique().on(t.userId, t.folderId, t.order).nullsNotDistinct()],
);

export const PostContentSnapshots = pgTable(
  'post_content_snapshots',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId('PCSN')),
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

export const PostContentStates = pgTable('post_content_states', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId('PCST')),
  postId: text('post_id')
    .notNull()
    .unique()
    .references(() => Posts.id, { onUpdate: 'cascade', onDelete: 'restrict' }),
  update: bytea('update').notNull(),
  vector: bytea('vector').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
  updatedAt: datetime('updated_at')
    .notNull()
    .default(sql`now()`),
});

export const PreorderPayments = pgTable('preorder_payments', {
  id: text('id')
    .primaryKey()
    .$defaultFn(() => createDbId('PP')),
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
    .$defaultFn(() => createDbId('PU')),
  email: text('email').unique().notNull(),
  wish: text('wish'),
  preorderPaymentId: text('preorder_payment_id').notNull(),
  createdAt: datetime('created_at')
    .notNull()
    .default(sql`now()`),
});

export const Users = pgTable(
  'users',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId('U', { length: 'short' })),
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

export const UserSessions = pgTable(
  'user_sessions',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId('USES')),
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
      .$defaultFn(() => createDbId('USSO')),
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
