import { drizzle } from 'drizzle-orm/bun-sql';
import { env } from '@/env';
import { DrizzleLogger } from './logger';
import * as enums from './schemas/enums';
import * as tables from './schemas/tables';
import type { PgDatabase, PgTransaction } from 'drizzle-orm/pg-core';

export const sql = new Bun.SQL({
  url: env.DATABASE_URL,
  max: 100,
  tls: { rejectUnauthorized: false },
  connection: {
    statement_timeout: 600_000,
    lock_timeout: 600_000,
  },
});

export const db = drizzle(sql, {
  schema: { ...tables, ...enums },
  logger: new DrizzleLogger(),
});

export type Database = typeof db;
export type Transaction = Database extends PgDatabase<infer T, infer U, infer V> ? PgTransaction<T, U, V> : never;

export * from './schemas/codes';
export * from './schemas/id';
export * from './schemas/tables';
export * from './utils';
