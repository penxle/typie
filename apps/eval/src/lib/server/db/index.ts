import { drizzle } from 'drizzle-orm/d1';
import * as schema from './schema.ts';
import type { D1Database } from '@cloudflare/workers-types';

export const createDb = (d1: D1Database) => drizzle(d1, { schema });

// D1은 문장당 바인딩 파라미터 100개 제한 — 대량 IN 조회는 청크로 나눠 합친다.
export const selectInChunks = async <T>(ids: string[], select: (chunk: string[]) => Promise<T[]>): Promise<T[]> => {
  const rows: T[] = [];
  for (let i = 0; i < ids.length; i += 100) {
    rows.push(...(await select(ids.slice(i, i + 100))));
  }
  return rows;
};

export * from './schema.ts';
