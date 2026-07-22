import { drizzle } from 'drizzle-orm/d1';
import * as schema from './schema.ts';
import type { D1Database } from '@cloudflare/workers-types';

export const createDb = (d1: D1Database) => drizzle(d1, { schema });
export * from './schema.ts';
