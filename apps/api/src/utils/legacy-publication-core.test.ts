import assert from 'node:assert/strict';
import test from 'node:test';
import { DocumentType, PublicationState, SiteDateDisplay, SpaceDateDisplay, SpaceState } from '@typie/lib/enums';
import { PgDialect } from 'drizzle-orm/pg-core';
import {
  buildLegacyPublicationValues,
  buildLegacyPublicDocumentsQuery,
  buildLegacySpaceValues,
  classifyLegacyDocument,
  isDemotedClassification,
  mapSiteDateDisplay,
  resolveLegacySpaceSlot,
  toLegacyPublicDocument,
} from './legacy-publication-core.ts';
import type { LegacyPublicDocumentRow } from './legacy-publication-core.ts';

const row: LegacyPublicDocumentRow = {
  document_id: 'D0A',
  entity_id: 'E0A',
  site_id: 'S0A',
  site_slug: 'lamp',
  site_name: '램프 아래서',
  site_logo_id: 'IMG0A',
  site_date_display: SiteDateDisplay.UPDATED_AT,
  document_type: DocumentType.NORMAL,
  character_count: 120,
  chain_public: true,
  title: '첫 글',
  subtitle: null,
  thumbnail_id: null,
  created_at: '2025-03-01 09:00:00+00',
  updated_at: '2025-04-02 10:30:00+00',
};

const document = toLegacyPublicDocument(row);
const emptyContext = { slugTakenSiteIds: new Set<string>(), publishedDocumentIds: new Set<string>() };

test('legacy public documents query walks the ancestor chain and joins the document tables', () => {
  const query = new PgDialect().sqlToQuery(buildLegacyPublicDocumentsQuery());
  assert.match(query.sql, /with recursive chain as/);
  assert.match(query.sql, /\(c\.ok and p\.visibility = \$5 and p\.state = \$6\)/);
  assert.match(query.sql, /bool_and\(ok\) as chain_public/);
  assert.match(query.sql, /from "entities" e/);
  assert.match(query.sql, /join "sites" s on s\.id = e\.site_id/);
  assert.match(query.sql, /join "documents" d on d\.entity_id = e\.id/);
  assert.match(query.sql, /left join "document_states" ds on ds\.document_id = d\.id/);
  assert.match(query.sql, /order by s\.id, d\.id/);
  assert.deepEqual(query.params, ['ACTIVE', 'ACTIVE', 'DOCUMENT', 'PUBLIC', 'PUBLIC', 'ACTIVE']);
});

test('raw rows map to camel case without touching the driver date strings', () => {
  assert.deepEqual(document, {
    documentId: 'D0A',
    entityId: 'E0A',
    siteId: 'S0A',
    siteSlug: 'lamp',
    siteName: '램프 아래서',
    siteLogoId: 'IMG0A',
    siteDateDisplay: SiteDateDisplay.UPDATED_AT,
    documentType: DocumentType.NORMAL,
    characterCount: 120,
    chainPublic: true,
    title: '첫 글',
    subtitle: null,
    thumbnailId: null,
    createdAt: '2025-03-01 09:00:00+00',
    updatedAt: '2025-04-02 10:30:00+00',
  });
});

test('template wins over every other classification', () => {
  const template = { ...document, documentType: DocumentType.TEMPLATE, characterCount: 0, chainPublic: false };
  assert.equal(classifyLegacyDocument(template, emptyContext), 'template');
});

test('empty body wins over a broken ancestor chain', () => {
  assert.equal(classifyLegacyDocument({ ...document, characterCount: 0, chainPublic: false }, emptyContext), 'empty');
});

test('a non-public ancestor demotes the document', () => {
  assert.equal(classifyLegacyDocument({ ...document, chainPublic: false }, emptyContext), 'ancestor');
});

test('a taken space slug skips the whole site before the publication check', () => {
  const context = { slugTakenSiteIds: new Set(['S0A']), publishedDocumentIds: new Set(['D0A']) };
  assert.equal(classifyLegacyDocument(document, context), 'slug_taken');
});

test('an existing publication row is reported instead of migrated', () => {
  const context = { slugTakenSiteIds: new Set<string>(), publishedDocumentIds: new Set(['D0A']) };
  assert.equal(classifyLegacyDocument(document, context), 'already_published');
});

test('a public normal document with a public chain migrates', () => {
  assert.equal(classifyLegacyDocument(document, emptyContext), 'migrated');
});

test('only template, empty and ancestor are demoted', () => {
  assert.equal(isDemotedClassification('template'), true);
  assert.equal(isDemotedClassification('empty'), true);
  assert.equal(isDemotedClassification('ancestor'), true);
  assert.equal(isDemotedClassification('slug_taken'), false);
  assert.equal(isDemotedClassification('already_published'), false);
  assert.equal(isDemotedClassification('migrated'), false);
});

test('site date display maps onto the space enum', () => {
  assert.equal(mapSiteDateDisplay(SiteDateDisplay.NONE), SpaceDateDisplay.NONE);
  assert.equal(mapSiteDateDisplay(SiteDateDisplay.UPDATED_AT), SpaceDateDisplay.UPDATED_AT);
  assert.equal(mapSiteDateDisplay(SiteDateDisplay.CREATED_AT), SpaceDateDisplay.PUBLISHED_AT);
});

test('space slot resolves to create, reuse or taken', () => {
  assert.deepEqual(resolveLegacySpaceSlot({ siteId: 'S0A' }, null), { kind: 'create' });
  assert.deepEqual(resolveLegacySpaceSlot({ siteId: 'S0A' }, { id: 'SPC0A', siteId: 'S0A', state: SpaceState.ACTIVE }), {
    kind: 'reuse',
    spaceId: 'SPC0A',
  });
  assert.deepEqual(resolveLegacySpaceSlot({ siteId: 'S0A' }, { id: 'SPC0A', siteId: 'S0A', state: SpaceState.DELETED }), { kind: 'taken' });
  assert.deepEqual(resolveLegacySpaceSlot({ siteId: 'S0A' }, { id: 'SPC0B', siteId: 'S0B', state: SpaceState.ACTIVE }), { kind: 'taken' });
});

test('space values copy the site identity and open both exposure switches', () => {
  assert.deepEqual(buildLegacySpaceValues(document), {
    siteId: 'S0A',
    slug: 'lamp',
    name: '램프 아래서',
    logoId: 'IMG0A',
    allowIndexing: true,
    allowDiscovery: true,
    dateDisplay: SpaceDateDisplay.UPDATED_AT,
  });
});

test('publication values publish immediately with the document timestamps', () => {
  const values = buildLegacyPublicationValues({
    documentId: 'D0A',
    spaceId: 'SPC0A',
    permalink: '12345678901',
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  });
  assert.equal(values.documentId, 'D0A');
  assert.equal(values.spaceId, 'SPC0A');
  assert.equal(values.permalink, '12345678901');
  assert.equal(values.state, PublicationState.PUBLISHED);
  assert.equal(values.publishedAt?.toISOString(), '2025-03-01T09:00:00.000Z');
  assert.equal(values.updatedAt?.toISOString(), '2025-04-02T10:30:00.000Z');
});
