import { randomUUID } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';

export const QUERY_RESULT_DIR = '/tmp/queries';

const MAX_INLINE_BYTES = 100 * 1024;
const PREVIEW_ROWS = 20;

type QueryResult = {
  success: boolean;
  count?: number;
  rows?: unknown[];
  error?: string;
};

export type QueryOutcome = {
  success: boolean;
  text: string;
};

export const runQuery = async (apiBaseUrl: string, apiSecret: string, sqlQuery: string): Promise<QueryOutcome> => {
  const res = await fetch(`${apiBaseUrl}/bmo/query`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${apiSecret}` },
    body: JSON.stringify({ query: sqlQuery }),
  });

  const raw = await res.text();
  const text = raw.trim();

  if (!res.ok) {
    return { success: false, text: JSON.stringify({ success: false, error: `API error ${res.status}: ${text}` }) };
  }

  let result: QueryResult;
  try {
    result = JSON.parse(text) as QueryResult;
  } catch {
    return { success: false, text: JSON.stringify({ success: false, error: `응답을 해석할 수 없습니다: ${text.slice(0, 500)}` }) };
  }

  if (!result.success) {
    return { success: false, text: JSON.stringify(result) };
  }

  const serialized = JSON.stringify(result);
  if (serialized.length <= MAX_INLINE_BYTES) {
    return { success: true, text: serialized };
  }

  const rows = result.rows ?? [];
  mkdirSync(QUERY_RESULT_DIR, { recursive: true });

  const path = `${QUERY_RESULT_DIR}/${randomUUID()}.json`;
  writeFileSync(path, JSON.stringify(rows));

  return {
    success: true,
    text: JSON.stringify({
      success: true,
      count: result.count,
      truncated: true,
      preview: rows.slice(0, PREVIEW_ROWS),
      path,
      note: `결과가 커서 전체 ${result.count}행을 ${path}에 JSON 배열로 저장했습니다. 상위 ${Math.min(PREVIEW_ROWS, rows.length)}행만 미리보기로 포함했습니다.`,
    }),
  };
};

let dbSchema: unknown | null = null;

export const getDatabaseSchema = async (apiBaseUrl: string, apiSecret: string): Promise<unknown> => {
  if (!dbSchema) {
    const res = await fetch(`${apiBaseUrl}/bmo/schema`, {
      headers: { Authorization: `Bearer ${apiSecret}` },
    });

    if (!res.ok) {
      throw new Error(`Schema API error ${res.status}: ${await res.text()}`);
    }

    dbSchema = await res.json();
  }

  return dbSchema;
};
