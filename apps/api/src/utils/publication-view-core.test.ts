import assert from 'node:assert/strict';
import test from 'node:test';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildIndexableSiteSlugsQuery,
  buildPinnedPublicationsQuery,
  buildPublishedPublicationByIdQuery,
  buildPublishedPublicationByNumberQuery,
  buildPublishedPublicationNumbersQuery,
  buildPublishedPublicationsByEntityIdsQuery,
  buildPublishedPublicationsQuery,
  buildReactionCountsQuery,
  buildSiteBySlugQuery,
  buildSitemapPaths,
  buildSiteTagQuery,
  buildSiteTagsQuery,
  clampPageSize,
  deriveExcerpt,
  SITE_PAGE_SIZE,
  SITE_PAGE_SIZE_MAX,
  toPublicationsPage,
} from './publication-view-core.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

test('site by slug requires an active site', () => {
  const query = buildSiteBySlugQuery(database, { slug: 'mysite' }).toSQL();
  assert.match(query.sql, /"sites"\."slug" = /);
  assert.match(query.sql, /"sites"\."state" = /);
  assert.deepEqual(query.params, ['mysite', 'ACTIVE']);
});

test('indexable site slugs require an active site that allows indexing and has a published row, oldest first', () => {
  const query = buildIndexableSiteSlugsQuery(database).toSQL();
  assert.match(query.sql, /"sites"\."state" = /);
  assert.match(query.sql, /"sites"\."allow_indexing" = /);
  assert.match(query.sql, /exists \(select "id" from "publications"/);
  assert.match(query.sql, /order by "sites"\."created_at"/);
  assert.deepEqual(query.params, ['ACTIVE', true, 'PUBLISHED']);
});

test('published publications join active entities, order by published_at desc then id desc, and cut by keyset cursor', () => {
  const query = buildPublishedPublicationsQuery(database, { siteId: 'S0A', after: 'PUB0Z', limit: 21 }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."site_id" = /);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /\("publications"\."published_at", "publications"\."id"\) < \(select/);
  assert.match(query.sql, /order by "publications"\."published_at" desc, "publications"\."id" desc/);
  assert.ok(query.params.includes('PUB0Z'));
  assert.equal(query.params.at(-1), 21);
});

test('published publications can be narrowed to one tag, one number, or an entity id list', () => {
  const byTag = buildPublishedPublicationsQuery(database, { siteId: 'S0A', tagName: '에세이', after: null, limit: 21 }).toSQL();
  assert.match(byTag.sql, /inner join "publication_tags"/);
  assert.match(byTag.sql, /"publication_tags"\."name" = /);

  const byNumber = buildPublishedPublicationsQuery(database, { siteId: 'S0A', number: '12345678901', after: null, limit: 1 }).toSQL();
  assert.match(byNumber.sql, /"entities"\."number" = /);
  assert.ok(byNumber.params.includes('12345678901'));

  const byEntities = buildPublishedPublicationsQuery(database, { siteId: 'S0A', entityIds: ['E1', 'E2'], after: null, limit: 10 }).toSQL();
  assert.match(byEntities.sql, /"entities"\."id" in \(/);
});

test('the home publication list drops pinned rows only when excludePinned is set', () => {
  const excluded = buildPublishedPublicationsQuery(database, { siteId: 'S0A', excludePinned: true, after: null, limit: 21 }).toSQL();
  assert.match(excluded.sql, /"publications"\."pinned_order" is null/);

  const included = buildPublishedPublicationsQuery(database, { siteId: 'S0A', after: null, limit: 21 }).toSQL();
  assert.doesNotMatch(included.sql, /"publications"\."pinned_order" is null/);
});

test('publication numbers and entity id lookups keep the published scope', () => {
  const numbers = buildPublishedPublicationNumbersQuery(database, { siteId: 'S0A', limit: 5000 }).toSQL();
  assert.match(numbers.sql, /^select "entities"\."number" from "publications"/);
  assert.match(numbers.sql, /"publications"\."site_id" = /);
  assert.match(numbers.sql, /"publications"\."state" = /);

  const byEntities = buildPublishedPublicationsByEntityIdsQuery(database, { entityIds: ['E1'] }).toSQL();
  assert.match(byEntities.sql, /"entities"\."id" in \(/);
  assert.match(byEntities.sql, /"publications"\."state" = /);
  assert.match(byEntities.sql, /"entities"\."state" = /);
});

test('pinned publications take a site list, published scope, and order by pinned order', () => {
  const query = buildPinnedPublicationsQuery(database, { siteIds: ['S0A'] }).toSQL();
  assert.match(query.sql, /"publications"\."site_id" in \(/);
  assert.match(query.sql, /"publications"\."pinned_order" is not null/);
  assert.match(query.sql, /order by "publications"\."site_id" asc, "publications"\."pinned_order" asc/);
});

test('site tags aggregate published rows by name, most used first', () => {
  const query = buildSiteTagsQuery(database, { siteId: 'S0A' }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /group by "publication_tags"\."name"/);
  assert.match(query.sql, /order by count\(\*\) desc, "publication_tags"\."name" asc/);
  assert.deepEqual(query.params, ['S0A', 'PUBLISHED', 'ACTIVE']);
});

test('a single site tag is aggregated by name instead of scanning every tag', () => {
  const query = buildSiteTagQuery(database, { siteId: 'S0A', name: '에세이' }).toSQL();
  assert.match(query.sql, /"publication_tags"\."name" = /);
  assert.match(query.sql, /group by "publication_tags"\."name"/);
  assert.deepEqual(query.params, ['S0A', 'PUBLISHED', 'ACTIVE', '에세이']);
});

test('a publication looked up by id or number is scoped to published rows on active entities', () => {
  const byId = buildPublishedPublicationByIdQuery(database, { publicationId: 'PUB0A' }).toSQL();
  assert.match(byId.sql, /"publications"\."id" = /);
  assert.deepEqual(byId.params, ['PUB0A', 'PUBLISHED', 'ACTIVE']);

  const byNumber = buildPublishedPublicationByNumberQuery(database, { number: '12345678901' }).toSQL();
  assert.match(byNumber.sql, /"entities"\."number" = /);
  assert.deepEqual(byNumber.params, ['12345678901', 'PUBLISHED', 'ACTIVE']);
});

test('page size clamps to the allowed range and pages report hasMore from the extra row', () => {
  assert.equal(clampPageSize(undefined), SITE_PAGE_SIZE);
  assert.equal(clampPageSize(0), 1);
  assert.equal(clampPageSize(999), SITE_PAGE_SIZE_MAX);
  assert.deepEqual(toPublicationsPage([1, 2, 3], 2), { publications: [1, 2], hasMore: true });
  assert.deepEqual(toPublicationsPage([1, 2], 2), { publications: [1, 2], hasMore: false });
});

test('excerpt prefers the override and otherwise collapses whitespace to 200 characters', () => {
  assert.equal(deriveExcerpt('본문', '미리보기'), '미리보기');
  assert.equal(deriveExcerpt('  첫  줄\n둘째 줄 ', null), '첫 줄 둘째 줄');
  assert.equal(deriveExcerpt('가'.repeat(250), '  '), '가'.repeat(200) + '...');
});

test('sitemap paths cover home, visible folders, published posts and encoded tags', () => {
  assert.deepEqual(
    buildSitemapPaths({ publicationNumbers: ['12345678901'], folderNumbers: ['98765432109'], tagNames: ['에세이', 'a b'] }),
    ['/', '/f/98765432109', '/p/12345678901', '/t/%EC%97%90%EC%84%B8%EC%9D%B4', '/t/a%20b'],
  );
});

test('reaction counts group reactions by document', () => {
  const query = buildReactionCountsQuery(database, { documentIds: ['DOC0A'] }).toSQL();
  assert.match(query.sql, /from "document_reactions"/);
  assert.match(query.sql, /"document_reactions"\."document_id" in \(/);
  assert.match(query.sql, /group by "document_reactions"\."document_id"/);
  assert.deepEqual(query.params, ['DOC0A']);
});
