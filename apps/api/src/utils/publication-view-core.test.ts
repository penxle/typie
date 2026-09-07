import assert from 'node:assert/strict';
import test from 'node:test';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildCollectionNeighborQuery,
  buildPublishedCollectionIdsQuery,
  buildPublishedPublicationByIdQuery,
  buildPublishedPublicationsQuery,
  buildSitemapPaths,
  buildSpaceBySlugQuery,
  buildSpaceTagQuery,
  buildSpaceTagsQuery,
  clampPageSize,
  deriveExcerpt,
  SPACE_PAGE_SIZE,
  SPACE_PAGE_SIZE_MAX,
  toPublicationsPage,
} from './publication-view-core.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

test('space by slug requires an active space on an active site', () => {
  const query = buildSpaceBySlugQuery(database, { slug: 'myspace' }).toSQL();
  assert.match(query.sql, /inner join "sites"/);
  assert.match(query.sql, /"spaces"\."slug" = /);
  assert.match(query.sql, /"spaces"\."state" = /);
  assert.match(query.sql, /"sites"\."state" = /);
  assert.deepEqual(query.params, ['myspace', 'ACTIVE', 'ACTIVE']);
});

test('published publications join active entities, order by published_at desc then id desc, and cut by keyset cursor', () => {
  const query = buildPublishedPublicationsQuery(database, { spaceId: 'SPC0A', after: 'PUB0Z', limit: 21 }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /\("publications"\."published_at", "publications"\."id"\) < \(select/);
  assert.match(query.sql, /order by "publications"\."published_at" desc, "publications"\."id" desc/);
  assert.match(query.sql, /limit /);
  assert.ok(query.params.includes('PUB0Z'));
  assert.equal(query.params.at(-1), 21);
});

test('published publications can be narrowed to one collection, one tag, or one id', () => {
  const byCollection = buildPublishedPublicationsQuery(database, {
    spaceId: 'SPC0A',
    collectionId: 'COL0A',
    after: null,
    limit: 100,
  }).toSQL();
  assert.match(byCollection.sql, /"publications"\."collection_id" = /);
  assert.match(byCollection.sql, /order by "publications"\."collection_order" asc/);

  const byTag = buildPublishedPublicationsQuery(database, { spaceId: 'SPC0A', tagName: '에세이', after: null, limit: 21 }).toSQL();
  assert.match(byTag.sql, /inner join "publication_tags"/);
  assert.match(byTag.sql, /"publication_tags"\."name" = /);

  const byId = buildPublishedPublicationsQuery(database, { spaceId: 'SPC0A', publicationId: 'PUB0A', after: null, limit: 1 }).toSQL();
  assert.match(byId.sql, /"publications"\."id" = /);
});

test('the home publication list drops pinned rows only when excludePinned is set', () => {
  const excluded = buildPublishedPublicationsQuery(database, { spaceId: 'SPC0A', excludePinned: true, after: null, limit: 21 }).toSQL();
  assert.match(excluded.sql, /"publications"\."pinned_order" is null/);

  const included = buildPublishedPublicationsQuery(database, { spaceId: 'SPC0A', after: null, limit: 21 }).toSQL();
  assert.doesNotMatch(included.sql, /"publications"\."pinned_order" is null/);
});

test('collection neighbours look on either side of the current order among published rows', () => {
  const prev = buildCollectionNeighborQuery(database, { collectionId: 'COL0A', collectionOrder: 'a1', direction: 'prev' }).toSQL();
  assert.match(prev.sql, /inner join "documents"/);
  assert.match(prev.sql, /inner join "entities"/);
  assert.match(prev.sql, /"publications"\."state" = /);
  assert.match(prev.sql, /"entities"\."state" = /);
  assert.match(prev.sql, /"publications"\."collection_order" < /);
  assert.match(prev.sql, /order by "publications"\."collection_order" desc/);
  assert.match(prev.sql, /limit /);
  const next = buildCollectionNeighborQuery(database, { collectionId: 'COL0A', collectionOrder: 'a1', direction: 'next' }).toSQL();
  assert.match(next.sql, /"publications"\."collection_order" > /);
  assert.match(next.sql, /order by "publications"\."collection_order" asc/);
});

test('space tags aggregate published rows by name, most used first', () => {
  const query = buildSpaceTagsQuery(database, { spaceId: 'SPC0A' }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /group by "publication_tags"\."name"/);
  assert.match(query.sql, /order by count\(\*\) desc, "publication_tags"\."name" asc/);
  assert.deepEqual(query.params, ['SPC0A', 'PUBLISHED', 'ACTIVE']);
});

test('a single space tag is aggregated by name instead of scanning every tag', () => {
  const query = buildSpaceTagQuery(database, { spaceId: 'SPC0A', name: '에세이' }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /"publication_tags"\."name" = /);
  assert.match(query.sql, /group by "publication_tags"\."name"/);
  assert.deepEqual(query.params, ['SPC0A', 'PUBLISHED', 'ACTIVE', '에세이']);
});

test('published collection ids are distinct collections with at least one published row', () => {
  const query = buildPublishedCollectionIdsQuery(database, { spaceId: 'SPC0A' }).toSQL();
  assert.match(query.sql, /select distinct "publications"\."collection_id"/);
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /"publications"\."collection_id" is not null/);
});

test('a publication looked up by id is scoped to published rows on active entities', () => {
  const query = buildPublishedPublicationByIdQuery(database, { publicationId: 'PUB0A' }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."id" = /);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.deepEqual(query.params, ['PUB0A', 'PUBLISHED', 'ACTIVE']);
});

test('page size clamps to the allowed range and pages report hasMore from the extra row', () => {
  assert.equal(clampPageSize(undefined), SPACE_PAGE_SIZE);
  assert.equal(clampPageSize(0), 1);
  assert.equal(clampPageSize(999), SPACE_PAGE_SIZE_MAX);
  assert.deepEqual(toPublicationsPage([1, 2, 3], 2), { publications: [1, 2], hasMore: true });
  assert.deepEqual(toPublicationsPage([1, 2], 2), { publications: [1, 2], hasMore: false });
});

test('excerpt prefers the override and otherwise collapses whitespace to 200 characters', () => {
  assert.equal(deriveExcerpt('본문', '미리보기'), '미리보기');
  assert.equal(deriveExcerpt('  첫  줄\n둘째 줄 ', null), '첫 줄 둘째 줄');
  assert.equal(deriveExcerpt('가'.repeat(250), '  '), '가'.repeat(200) + '...');
});

test('sitemap paths cover home, published posts, series and encoded tags', () => {
  assert.deepEqual(buildSitemapPaths({ publicationIds: ['PUB0A'], collectionIds: ['COL0A'], tagNames: ['에세이', 'a b'] }), [
    '/',
    '/p/PUB0A',
    '/s/COL0A',
    '/t/%EC%97%90%EC%84%B8%EC%9D%B4',
    '/t/a%20b',
  ]);
});
