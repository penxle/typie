import { drizzle } from 'drizzle-orm/node-postgres';
import { Pool } from 'pg';
import { env } from '@/env';
import { DrizzleLogger } from './logger';
import * as enums from './schemas/enums';
import * as tables from './schemas/tables';
import type { PgDatabase, PgTransaction } from 'drizzle-orm/pg-core';

export const pool = new Pool({
  connectionString: env.DATABASE_URL,
  ssl: { rejectUnauthorized: false },
  max: 200,
  idleTimeoutMillis: 30 * 60 * 1000,
  statement_timeout: 10 * 60 * 1000,
  lock_timeout: 10 * 60 * 1000,
});

export const db = drizzle(pool, {
  schema: { ...tables, ...enums },
  logger: new DrizzleLogger(),
});

export type Database = typeof db;
export type Transaction = Database extends PgDatabase<infer T, infer U, infer V> ? PgTransaction<T, U, V> : never;

export * from './schemas/codes';
export * from './schemas/id';
export * from './schemas/tables';
export * from './utils';
