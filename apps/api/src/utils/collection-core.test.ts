import assert from 'node:assert/strict';
import test from 'node:test';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import { buildCollectionsBySpaceQuery, buildPinnedPublicationsQuery, canPinMore, PIN_LIMIT } from './collection-core.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

test('pin limit is three', () => {
  assert.equal(PIN_LIMIT, 3);
  assert.equal(canPinMore(2), true);
  assert.equal(canPinMore(3), false);
});

test('collections by space are ordered by creation', () => {
  const query = buildCollectionsBySpaceQuery(database, { spaceIds: ['SPC0A'] }).toSQL();
  assert.match(query.sql, /"collections"\."space_id" in \(/);
  assert.match(query.sql, /order by "collections"\."created_at" asc/);
});

test('pinned publications are published rows on active entities with pinned order', () => {
  const query = buildPinnedPublicationsQuery(database, { spaceIds: ['SPC0A'] }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."pinned_order" is not null/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /order by "publications"\."space_id" asc, "publications"\."pinned_order" asc/);
  assert.deepEqual(query.params, ['SPC0A', 'PUBLISHED', 'ACTIVE']);
});
