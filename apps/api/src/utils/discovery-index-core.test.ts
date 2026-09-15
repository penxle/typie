import assert from 'node:assert/strict';
import test from 'node:test';
import { EntityState, PublicationState, SiteState } from '@typie/lib/enums';
import dayjs from 'dayjs';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildPublicationIndexRowsQuery,
  buildPublicationTagNamesQuery,
  buildSiteIndexRowsQuery,
  buildSiteTagNamesQuery,
  collectTagNames,
  toPublicationIndexDocument,
  toSiteIndexDocument,
  toTagIndexDocument,
} from './discovery-index-core.ts';
import type { Database } from '#/db/index.ts';
import type { PublicationIndexRow } from './discovery-index-core.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

const publishedAt = dayjs('2026-09-01T00:00:00Z');

const row = (overrides: Partial<PublicationIndexRow> = {}): PublicationIndexRow => ({
  id: 'PUB0A',
  siteId: 'S0A',
  state: PublicationState.PUBLISHED,
  publishedAt,
  entityState: EntityState.ACTIVE,
  siteState: SiteState.ACTIVE,
  allowDiscovery: true,
  password: null,
  ...overrides,
});

const version = { title: '첫 글', subtitle: null, text: '본문입니다' };

test('a published, active, password-free publication becomes an index document with decomposed fields and tags', () => {
  const doc = toPublicationIndexDocument({ row: row(), version, tagNames: ['에세이', '단편'] });
  assert.ok(doc);
  assert.equal(doc.site_id, 'S0A');
  assert.equal(doc.discoverable, true);
  assert.equal(doc.title, '첫 글');
  assert.equal(doc.title_decomposed, 'ㅊㅓㅅ ㄱㅡㄹ');
  assert.equal(doc.subtitle, null);
  assert.equal(doc.subtitle_decomposed, null);
  assert.equal(doc.text, '본문입니다');
  assert.deepEqual(doc.tags, ['에세이', '단편']);
  assert.equal(doc.tags_text, '에세이 단편');
  assert.equal(doc.published_at, publishedAt);
});

test('discoverable follows the discovery switch without dropping the document', () => {
  assert.equal(toPublicationIndexDocument({ row: row({ allowDiscovery: false }), version, tagNames: [] })?.discoverable, false);
});

test('unpublished, inactive, password-protected, or versionless publications are not indexed', () => {
  assert.equal(toPublicationIndexDocument({ row: row({ state: PublicationState.UNPUBLISHED }), version, tagNames: [] }), null);
  assert.equal(toPublicationIndexDocument({ row: row({ state: PublicationState.SCHEDULED }), version, tagNames: [] }), null);
  assert.equal(toPublicationIndexDocument({ row: row({ entityState: EntityState.DELETED }), version, tagNames: [] }), null);
  assert.equal(toPublicationIndexDocument({ row: row({ siteState: SiteState.DELETED }), version, tagNames: [] }), null);
  assert.equal(toPublicationIndexDocument({ row: row({ password: '' }), version, tagNames: [] }), null);
  assert.equal(toPublicationIndexDocument({ row: row({ password: '1234' }), version, tagNames: [] }), null);
  assert.equal(toPublicationIndexDocument({ row: row(), version: undefined, tagNames: [] }), null);
});

test('a site document exists only for active, discoverable sites', () => {
  const base = { id: 'S0A', name: '내 스페이스', description: null, state: SiteState.ACTIVE, allowDiscovery: true };
  assert.deepEqual(toSiteIndexDocument(base), { name: '내 스페이스', name_decomposed: 'ㄴㅐ ㅅㅡㅍㅔㅇㅣㅅㅡ', description: null });
  assert.equal(toSiteIndexDocument({ ...base, allowDiscovery: false }), null);
  assert.equal(toSiteIndexDocument({ ...base, state: SiteState.DELETED }), null);
});

test('tag documents carry the decomposed name and count, and tag name collection dedupes across lists', () => {
  assert.deepEqual(toTagIndexDocument({ name: '에세이', count: 3 }), { name: '에세이', name_decomposed: 'ㅇㅔㅅㅔㅇㅣ', count: 3 });
  assert.deepEqual(collectTagNames(['a', 'b'], undefined, ['b', 'c'], []), ['a', 'b', 'c']);
});

test('publication index rows join documents, entities, sites and take an id list', () => {
  const query = buildPublicationIndexRowsQuery(database, { publicationIds: ['PUB0A'] }).toSQL();
  assert.match(query.sql, /inner join "documents"/);
  assert.match(query.sql, /inner join "entities"/);
  assert.match(query.sql, /inner join "sites"/);
  assert.doesNotMatch(query.sql, /"spaces"/);
  assert.match(query.sql, /"publications"\."id" in \(/);
  assert.match(query.sql, /"documents"\."password"/);
  assert.match(query.sql, /"sites"\."allow_discovery"/);
  assert.deepEqual(query.params, ['PUB0A']);
});

test('tag name lookups take id lists and the site variant is distinct', () => {
  const byPublication = buildPublicationTagNamesQuery(database, { publicationIds: ['PUB0A'] }).toSQL();
  assert.match(byPublication.sql, /"publication_tags"\."publication_id" in \(/);
  assert.match(byPublication.sql, /order by "publication_tags"\."publication_id" asc, "publication_tags"\."order" asc/);

  const bySite = buildSiteTagNamesQuery(database, { siteIds: ['S0A'] }).toSQL();
  assert.match(bySite.sql, /select distinct "name" from "publication_tags"/);
  assert.match(bySite.sql, /"publication_tags"\."site_id" in \(/);

  const sites = buildSiteIndexRowsQuery(database, { siteIds: ['S0A'] }).toSQL();
  assert.match(sites.sql, /"sites"\."id" in \(/);
  assert.match(sites.sql, /"state", "allow_discovery" from "sites"/);
});
