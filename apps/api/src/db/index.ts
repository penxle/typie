import { drizzle } from 'drizzle-orm/postgres-js';
import postgres from 'postgres';
import { dev, env } from '#/env.ts';
import { DrizzleLogger } from './logger.ts';
import * as enums from './schemas/enums.ts';
import * as tables from './schemas/tables.ts';
import type { PgDatabase, PgTransaction } from 'drizzle-orm/pg-core';

export const pg = postgres(env.DATABASE_URL, {
  max: dev ? 20 : 50,
  connect_timeout: 5,
  idle_timeout: 30,
  prepare: false,
  ssl: 'prefer',
});

export const db = drizzle(pg, {
  schema: { ...tables, ...enums },
  logger: new DrizzleLogger(),
});

export const pgr = postgres(env.DATABASE_RO_URL ?? env.DATABASE_URL, {
  max: dev ? 5 : 20,
  connect_timeout: 5,
  idle_timeout: 30,
  prepare: false,
  ssl: 'prefer',
});

export const dbr = drizzle(pgr, {
  schema: { ...tables, ...enums },
  logger: new DrizzleLogger(),
});

export type Database = typeof db;
export type Transaction = Database extends PgDatabase<infer T, infer U, infer V> ? PgTransaction<T, U, V> : never;

export * from './schemas/codes.ts';
export * from './schemas/id.ts';
export * from './schemas/tables.ts';
export * from './utils.ts';
