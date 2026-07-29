import { drizzle } from 'drizzle-orm/d1';
import * as schema from './schema.ts';
import type { D1Database } from '@cloudflare/workers-types';

export const createDb = (d1: D1Database) => drizzle(d1, { schema });
export type Db = ReturnType<typeof createDb>;

export * from './schema.ts';

// 같은 상한이 읽기에도 걸린다 — id 목록으로 조회할 때 목록이 100을 넘으면 쿼리가 통째로
// 실패한다(라운드 하나가 항목 1,000건을 넘기면 상세 화면이 아무것도 못 띄운다).
// 다른 조건이 함께 묶이는 자리를 위해 여유를 두고 자른다.
const IN_CHUNK = 90;

export const inChunks = async <K, R>(keys: K[], run: (chunk: K[]) => Promise<R[]>): Promise<R[]> => {
  if (keys.length === 0) return [];
  const out: R[] = [];
  for (let i = 0; i < keys.length; i += IN_CHUNK) {
    out.push(...(await run(keys.slice(i, i + IN_CHUNK))));
  }
  return out;
};

// D1은 문장당 바인딩 파라미터가 100개다. 행당 컬럼 수로 나눠 한 문장이 넘지 않게 쪼갠다.
export const chunkRows = <T>(rows: T[], columnsPerRow: number, run: (chunk: T[]) => void): void => {
  if (rows.length === 0) return;
  const size = Math.max(1, Math.floor(100 / columnsPerRow));
  for (let i = 0; i < rows.length; i += size) {
    run(rows.slice(i, i + size));
  }
};
