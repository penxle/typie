import assert from 'node:assert/strict';
import test from 'node:test';
import { EntityType, EntityVisibility } from '@typie/lib/enums';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildSiteTree,
  buildSiteTreeRowsQuery,
  descendantDocumentEntityIds,
  isPathFolder,
  pathAncestors,
  pathChildCounts,
  pathChildren,
  pathFolderIds,
  pathNeighbors,
} from './site-tree-core.ts';
import type { Database } from '#/db/index.ts';
import type { SiteTreeRow } from './site-tree-core.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

const row = (id: string, parentId: string | null, type: EntityType, visibility: EntityVisibility, order: string): SiteTreeRow => ({
  id,
  siteId: 'S0A',
  parentId,
  type,
  visibility,
  order,
});

const fixture = [
  row('F1', null, EntityType.FOLDER, EntityVisibility.PUBLIC, 'a'),
  row('D1', 'F1', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a'),
  row('D2', 'F1', EntityType.DOCUMENT, EntityVisibility.UNLISTED, 'b'),
  row('D3', 'F1', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'c'),
  row('F2', 'F1', EntityType.FOLDER, EntityVisibility.PUBLIC, 'd'),
  row('D4', 'F2', EntityType.DOCUMENT, EntityVisibility.PRIVATE, 'a'),
  row('F3', null, EntityType.FOLDER, EntityVisibility.UNLISTED, 'b'),
  row('F4', 'F3', EntityType.FOLDER, EntityVisibility.PUBLIC, 'a'),
  row('D5', 'F4', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a'),
  row('D8', 'F3', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'b'),
  row('F6', 'F3', EntityType.FOLDER, EntityVisibility.PRIVATE, 'c'),
  row('D9', 'F6', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a'),
  row('D6', null, EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'c'),
  row('D7', null, EntityType.DOCUMENT, EntityVisibility.PRIVATE, 'd'),
  row('F5', null, EntityType.FOLDER, EntityVisibility.PUBLIC, 'e'),
];

const ids = (rows: readonly SiteTreeRow[]) => rows.map((r) => r.id);

test('a document is visible only when published and a folder only when it has a published descendant', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(
    [...tree.visible].toSorted((a, b) => a.localeCompare(b)),
    ['D1', 'D3', 'D5', 'D6', 'D8', 'D9', 'F1', 'F3', 'F4', 'F6'],
  );
});

test('a path step is a visible folder that is published', () => {
  const tree = buildSiteTree(fixture);
  assert.equal(isPathFolder(tree, 'F1'), true);
  assert.equal(isPathFolder(tree, 'F4'), true);
  assert.equal(isPathFolder(tree, 'F3'), false);
  assert.equal(isPathFolder(tree, 'F6'), false);
  assert.equal(isPathFolder(tree, 'F2'), false);
  assert.equal(isPathFolder(tree, 'F5'), false);
  assert.equal(isPathFolder(tree, 'D1'), false);
});

test('hidden folders are flattened in place, through several hidden levels, keeping tree order', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(ids(pathChildren(tree, null)), ['F1', 'F4', 'D8', 'D9', 'D6']);
  assert.deepEqual(ids(pathChildren(tree, 'F1')), ['D1', 'D3']);
  assert.deepEqual(ids(pathChildren(tree, 'F4')), ['D5']);
  assert.deepEqual(pathChildren(tree, 'F3'), []);
  assert.deepEqual(pathChildren(tree, 'F5'), []);
});

test('ancestors keep only path steps, root first', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(ids(pathAncestors(tree, 'D1')), ['F1']);
  assert.deepEqual(ids(pathAncestors(tree, 'D5')), ['F4']);
  assert.deepEqual(ids(pathAncestors(tree, 'D8')), []);
  assert.deepEqual(ids(pathAncestors(tree, 'D9')), []);
  assert.deepEqual(ids(pathAncestors(tree, 'F4')), []);
  assert.deepEqual(ids(pathAncestors(tree, 'D6')), []);
});

test('neighbors are the published documents beside the current one among its path siblings', () => {
  const tree = buildSiteTree(fixture);
  const pick = (id: string) => Object.fromEntries(Object.entries(pathNeighbors(tree, id)).map(([k, v]) => [k, v?.id ?? null]));
  assert.deepEqual(pick('D1'), { prev: null, next: 'D3' });
  assert.deepEqual(pick('D3'), { prev: 'D1', next: null });
  assert.deepEqual(pick('D8'), { prev: null, next: 'D9' });
  assert.deepEqual(pick('D9'), { prev: 'D8', next: 'D6' });
  assert.deepEqual(pick('D6'), { prev: 'D9', next: null });
  assert.deepEqual(pick('D5'), { prev: null, next: null });
  assert.deepEqual(pathNeighbors(tree, 'D2'), { prev: null, next: null });
});

test('child counts and sitemap folder ids come from the path map', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(pathChildCounts(tree, 'F1'), { folders: 0, documents: 2 });
  assert.deepEqual(pathChildCounts(tree, 'F4'), { folders: 0, documents: 1 });
  assert.deepEqual(pathChildCounts(tree, 'F3'), { folders: 0, documents: 0 });
  assert.deepEqual(
    pathFolderIds(tree).toSorted((a, b) => a.localeCompare(b)),
    ['F1', 'F4'],
  );
});

test('descendant document ids include every document under the given folders regardless of visibility', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(
    descendantDocumentEntityIds(tree, ['F1']).toSorted((a, b) => a.localeCompare(b)),
    ['D1', 'D2', 'D3', 'D4'],
  );
  assert.deepEqual(
    descendantDocumentEntityIds(tree, ['F3', 'F5']).toSorted((a, b) => a.localeCompare(b)),
    ['D5', 'D8', 'D9'],
  );
});

test('a row whose parent is missing from the site rows is treated as a root', () => {
  const tree = buildSiteTree([row('D9', 'GONE', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a')]);
  assert.deepEqual(ids(pathChildren(tree, null)), ['D9']);
});

test('site tree rows query takes active non-divider entities of the given sites', () => {
  const query = buildSiteTreeRowsQuery(database, { siteIds: ['S0A'] }).toSQL();
  assert.match(query.sql, /"entities"\."site_id" in \(/);
  assert.match(query.sql, /"entities"\."state" in \(/);
  assert.match(query.sql, /"entities"\."type" <> /);
  assert.deepEqual(query.params, ['S0A', 'ACTIVE', 'DIVIDER']);
});
