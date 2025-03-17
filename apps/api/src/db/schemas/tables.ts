// spell-checker:ignoreRegExp /createDbId\('[A-Z]{1,4}'/g

import { eq, sql } from 'drizzle-orm';
import { index, integer, pgTable, text, uniqueIndex } from 'drizzle-orm/pg-core';
import * as E from './enums';
import { createDbId } from './id';
import { datetime, jsonb } from './types';

export const Jobs = pgTable(
  'jobs',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId('J', { length: 'long' })),
    lane: text('lane').notNull(),
    name: text('name').notNull(),
    payload: jsonb('payload').notNull(),
    retries: integer('retries').notNull().default(0),
    state: E._JobState('state').notNull().default('PENDING'),
    createdAt: datetime('created_at')
      .notNull()
      .default(sql`now()`),
    updatedAt: datetime('updated_at')
      .notNull()
      .default(sql`now()`),
  },
  (t) => [index().on(t.lane, t.state, t.createdAt)],
);

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
  createdAt: datetime('created_at'),
});

export const Users = pgTable(
  'users',
  {
    id: text('id')
      .primaryKey()
      .$defaultFn(() => createDbId('U', { length: 'short' })),
    email: text('email').notNull(),
    name: text('name').notNull(),
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
