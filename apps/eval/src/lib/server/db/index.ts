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

// 같은 제한이 다중 행 INSERT에도 걸린다: 바인딩 수는 컬럼 수 × 행 수다. 한 문장에 몰아넣으면
// 그 곱이 100을 넘는 순간 문장 전체가 실패한다 — 판정 15건(7컬럼 × 15 = 105)부터 통째로
// 저장되지 않던 사고가 실제로 있었다.
export const chunkRows = <T>(rows: T[], columns: number, emit: (chunk: T[]) => void): void => {
  const size = Math.max(1, Math.floor(100 / columns));
  for (let i = 0; i < rows.length; i += size) {
    emit(rows.slice(i, i + size));
  }
};

export * from './schema.ts';
