import assert from 'node:assert/strict';
import test from 'node:test';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildRecentDocumentsBatchQuery,
  clampRecentDocumentLimit,
  RECENT_DOCUMENT_LIMIT,
  toRecentDocumentsPage,
} from './recent-documents.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

const compile = (sort: 'VIEWED_AT' | 'UPDATED_AT') =>
  buildRecentDocumentsBatchQuery(database, { userId: 'U0OWNER', siteIds: ['S0SPACE'], sort, limit: 5 }).toSQL();

test('recently viewed documents are owner and site scoped, active documents with a view timestamp', () => {
  const query = compile('VIEWED_AT');

  assert.match(query.sql, /"entities"\."user_id"/);
  assert.match(query.sql, /"entities"\."site_id"/);
  assert.match(query.sql, /"entities"\."state"/);
  assert.match(query.sql, /"entities"\."type"/);
  assert.match(query.sql, /"entities"\."viewed_at" is not null/);
  assert.match(query.sql, /order by .*"entities"\."viewed_at" desc, .*"documents"\."id" desc/);
  assert.equal(query.params.at(-1), 6);
  assert.deepEqual(query.params.slice(0, 4), ['U0OWNER', 'S0SPACE', 'ACTIVE', 'DOCUMENT']);
});

test('recently modified documents use document updated time without requiring a view timestamp', () => {
  const query = compile('UPDATED_AT');

  assert.doesNotMatch(query.sql, /viewed_at is not null/);
  assert.match(query.sql, /order by .*"documents"\."updated_at" desc, .*"documents"\."id" desc/);
  assert.equal(query.params.at(-1), 6);
});

test('recent documents for multiple sites are ranked and limited per site in one query', () => {
  const query = buildRecentDocumentsBatchQuery(database, {
    userId: 'U0OWNER',
    siteIds: ['S0FIRST', 'S0SECOND'],
    sort: 'UPDATED_AT',
    limit: 5,
  }).toSQL();

  assert.match(query.sql, /row_number\(\) over \(partition by .*"entities"\."site_id"/);
  assert.match(query.sql, /"recent_rank" <=/);
  assert.deepEqual(query.params.slice(0, 3), ['U0OWNER', 'S0FIRST', 'S0SECOND']);
  assert.equal(query.params.at(-1), 6);
});

test('recent document pages return only the requested documents and expose a bounded lookahead', () => {
  assert.deepEqual(toRecentDocumentsPage([1, 2, 3, 4, 5, 6], 5), {
    documents: [1, 2, 3, 4, 5],
    hasMore: true,
  });
  assert.deepEqual(toRecentDocumentsPage([1, 2, 3], 5), {
    documents: [1, 2, 3],
    hasMore: false,
  });
});

test('recent document limits are clamped and do not query beyond the maximum page size', () => {
  assert.equal(clampRecentDocumentLimit(0), 1);
  assert.equal(clampRecentDocumentLimit(RECENT_DOCUMENT_LIMIT + 1), RECENT_DOCUMENT_LIMIT);

  const query = buildRecentDocumentsBatchQuery(database, {
    userId: 'U0OWNER',
    siteIds: ['S0SPACE'],
    sort: 'VIEWED_AT',
    limit: RECENT_DOCUMENT_LIMIT + 1,
  }).toSQL();

  assert.equal(query.params.at(-1), RECENT_DOCUMENT_LIMIT);
  assert.equal(toRecentDocumentsPage(Array.from({ length: RECENT_DOCUMENT_LIMIT + 1 }), RECENT_DOCUMENT_LIMIT + 1).hasMore, false);
});

test('recent documents hide entries dismissed after their latest activity', () => {
  const viewed = compile('VIEWED_AT');
  assert.match(viewed.sql, /\("entities"\."recent_dismissed_at" is null or "entities"\."viewed_at" > "entities"\."recent_dismissed_at"\)/);

  const updated = compile('UPDATED_AT');
  assert.match(
    updated.sql,
    /\("entities"\."recent_dismissed_at" is null or "documents"\."updated_at" > "entities"\."recent_dismissed_at"\)/,
  );
});
