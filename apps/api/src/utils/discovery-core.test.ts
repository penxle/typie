import assert from 'node:assert/strict';
import test from 'node:test';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildDiscoverablePublicationsByIdsQuery,
  buildDiscoverableSpacesByIdsQuery,
  buildDiscoveryFeedQuery,
  buildDiscoveryPublicationsQuery,
  buildDiscoveryRecentPublicationsQuery,
  buildDiscoveryTagCountsQuery,
  buildDiscoveryTagsQuery,
  DISCOVERY_RECENT_SCAN_LIMIT,
  DISCOVERY_TAG_LIMIT,
  pickFirstByKey,
} from './discovery-core.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

const assertDiscoverablePredicate = (sql: string) => {
  assert.match(sql, /inner join "documents"/);
  assert.match(sql, /inner join "entities"/);
  assert.match(sql, /inner join "spaces"/);
  assert.match(sql, /inner join "sites"/);
  assert.match(sql, /"publications"\."state" = /);
  assert.match(sql, /"entities"\."state" = /);
  assert.match(sql, /"spaces"\."state" = /);
  assert.match(sql, /"sites"\."state" = /);
  assert.match(sql, /"spaces"\."allow_indexing" = /);
  assert.match(sql, /"spaces"\."allow_discovery" = /);
  assert.match(sql, /"documents"\."password" is null/);
  assert.match(sql, /where .*"publications"\."state" = \$\d+ and .*"documents"\."password" is null/s);
};

test('discovery publications apply the platform predicate, order newest first, and cut by keyset cursor', () => {
  const query = buildDiscoveryPublicationsQuery(database, { after: 'PUB0Z', limit: 21 }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.doesNotMatch(query.sql, /"publications"\."space_id" = \$/);
  assert.match(query.sql, /\("publications"\."published_at", "publications"\."id"\) < \(select/);
  assert.match(query.sql, /order by "publications"\."published_at" desc, "publications"\."id" desc/);
  assert.match(query.sql, /limit /);
  assert.deepEqual(query.params, ['PUBLISHED', 'ACTIVE', 'ACTIVE', 'ACTIVE', true, true, 'PUB0Z', 21]);
});

test('discovery publications without a cursor omit the keyset clause', () => {
  const query = buildDiscoveryPublicationsQuery(database, { after: null, limit: 21 }).toSQL();
  assert.doesNotMatch(query.sql, /< \(select/);
  assert.deepEqual(query.params, ['PUBLISHED', 'ACTIVE', 'ACTIVE', 'ACTIVE', true, true, 21]);
});

test('discovery publications can be narrowed to one tag across every space', () => {
  const query = buildDiscoveryPublicationsQuery(database, { tagName: '에세이', after: null, limit: 21 }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.match(query.sql, /inner join "publication_tags"/);
  assert.match(query.sql, /"publication_tags"\."name" = /);
  assert.doesNotMatch(query.sql, /"publications"\."space_id" = \$/);
  assert.ok(query.params.includes('에세이'));
});

test('discovery feed keeps only the newest post of each consecutive same-space run, ordered newest first', () => {
  const query = buildDiscoveryFeedQuery(database, { after: null, limit: 21 }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.match(
    query.sql,
    /lag\("publications"\."space_id"\) over \(order by "publications"\."published_at" desc, "publications"\."id" desc\)/,
  );
  assert.match(query.sql, /"prev_space_id" is distinct from "feed_publications"\."space_id"/);
  assert.match(query.sql, /order by "feed_publications"\."published_at" desc, "feed_publications"\."id" desc/);
  assert.doesNotMatch(query.sql, /<= \(select/);
  assert.doesNotMatch(query.sql, /"feed_publications"\."id" <> /);
  assert.deepEqual(query.params, ['PUBLISHED', 'ACTIVE', 'ACTIVE', 'ACTIVE', true, true, 21]);
});

test('discovery feed cursor keeps the cursor row inside the window and drops it from the page', () => {
  const query = buildDiscoveryFeedQuery(database, { after: 'PUB0Z', limit: 21 }).toSQL();
  assert.match(query.sql, /\("publications"\."published_at", "publications"\."id"\) <= \(select/);
  assert.match(query.sql, /"feed_publications"\."id" <> \$\d+/);
  assert.match(query.sql, /limit /);
  assert.deepEqual(query.params, ['PUBLISHED', 'ACTIVE', 'ACTIVE', 'ACTIVE', true, true, 'PUB0Z', 'PUB0Z', 21]);
});

test('discovery tags aggregate discoverable rows by name, most used first, limited when asked', () => {
  const query = buildDiscoveryTagsQuery(database, { limit: DISCOVERY_TAG_LIMIT }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.match(query.sql, /inner join "publication_tags"/);
  assert.match(query.sql, /group by "publication_tags"\."name"/);
  assert.match(query.sql, /order by count\(\*\) desc, "publication_tags"\."name" asc/);
  assert.match(query.sql, /limit /);
  assert.equal(query.params.at(-1), 20);
  assert.equal(DISCOVERY_TAG_LIMIT, 20);
});

test('discovery tags without a limit scan every tag and a single tag filters by name', () => {
  const all = buildDiscoveryTagsQuery(database, {}).toSQL();
  assert.doesNotMatch(all.sql, /limit /);

  const one = buildDiscoveryTagsQuery(database, { name: '에세이', limit: 1 }).toSQL();
  assert.match(one.sql, /"publication_tags"\."name" = /);
  assert.match(one.sql, /group by "publication_tags"\."name"/);
  assert.ok(one.params.includes('에세이'));
  assert.equal(one.params.at(-1), 1);
});

test('discoverable publications by ids keep the platform predicate and take an id list', () => {
  const query = buildDiscoverablePublicationsByIdsQuery(database, { publicationIds: ['PUB0A', 'PUB0B'] }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.match(query.sql, /"publications"\."id" in \(/);
  assert.doesNotMatch(query.sql, /order by/);
  assert.ok(query.params.includes('PUB0A') && query.params.includes('PUB0B'));
});

test('discoverable spaces by ids require active space and site with both switches on', () => {
  const query = buildDiscoverableSpacesByIdsQuery(database, { spaceIds: ['SPC0A'] }).toSQL();
  assert.match(query.sql, /inner join "sites"/);
  assert.match(query.sql, /"spaces"\."id" in \(/);
  assert.match(query.sql, /"spaces"\."state" = /);
  assert.match(query.sql, /"sites"\."state" = /);
  assert.match(query.sql, /"spaces"\."allow_indexing" = /);
  assert.match(query.sql, /"spaces"\."allow_discovery" = /);
  assert.doesNotMatch(query.sql, /"publications"/);
  assert.deepEqual(query.params, ['SPC0A', 'ACTIVE', 'ACTIVE', true, true]);
});

test('discovery tag counts aggregate only the named tags under the platform predicate', () => {
  const query = buildDiscoveryTagCountsQuery(database, { names: ['에세이', '단편'] }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.match(query.sql, /"publication_tags"\."name" in \(/);
  assert.match(query.sql, /group by "publication_tags"\."name"/);
  assert.doesNotMatch(query.sql, /limit /);
  assert.ok(query.params.includes('에세이') && query.params.includes('단편'));
});

test('recent discovery publications scan the newest discoverable rows with space and series ids only', () => {
  const query = buildDiscoveryRecentPublicationsQuery(database, { limit: DISCOVERY_RECENT_SCAN_LIMIT }).toSQL();
  assertDiscoverablePredicate(query.sql);
  assert.match(query.sql, /^select "publications"\."id", "publications"\."space_id", "publications"\."collection_id" from/);
  assert.match(query.sql, /order by "publications"\."published_at" desc, "publications"\."id" desc/);
  assert.equal(query.params.at(-1), 200);
});

test('picking the first row per key keeps scan order, skips null keys and stops at the limit', () => {
  const rows = [
    { id: 'p1', key: 'a' },
    { id: 'p2', key: 'a' },
    { id: 'p3', key: null },
    { id: 'p4', key: 'b' },
    { id: 'p5', key: 'c' },
  ];
  assert.deepEqual(
    pickFirstByKey(rows, (row) => row.key, 2).map((row) => row.id),
    ['p1', 'p4'],
  );
  assert.deepEqual(
    pickFirstByKey(rows, (row) => row.key, 10).map((row) => row.id),
    ['p1', 'p4', 'p5'],
  );
});
