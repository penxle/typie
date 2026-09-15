import assert from 'node:assert/strict';
import test from 'node:test';
import { EntityType, EntityVisibility } from '@typie/lib/enums';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildSiteTree,
  buildSiteTreeRowsQuery,
  descendantDocumentEntityIds,
  visibleAncestors,
  visibleChildCounts,
  visibleChildren,
  visibleFolderIds,
  visibleNeighbors,
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
  row('F1', null, EntityType.FOLDER, EntityVisibility.PRIVATE, 'a'),
  row('D1', 'F1', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a'),
  row('D2', 'F1', EntityType.DOCUMENT, EntityVisibility.UNLISTED, 'b'),
  row('D3', 'F1', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'c'),
  row('F2', 'F1', EntityType.FOLDER, EntityVisibility.UNLISTED, 'd'),
  row('D4', 'F2', EntityType.DOCUMENT, EntityVisibility.PRIVATE, 'a'),
  row('F3', null, EntityType.FOLDER, EntityVisibility.UNLISTED, 'b'),
  row('F4', 'F3', EntityType.FOLDER, EntityVisibility.PRIVATE, 'a'),
  row('D5', 'F4', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a'),
  row('D6', null, EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'c'),
  row('D7', null, EntityType.DOCUMENT, EntityVisibility.PRIVATE, 'd'),
  row('F5', null, EntityType.FOLDER, EntityVisibility.UNLISTED, 'e'),
];

test('a document is visible only when published and a folder only when it has a published descendant', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(
    [...tree.visible].toSorted((a, b) => a.localeCompare(b)),
    ['D1', 'D3', 'D5', 'D6', 'F1', 'F3', 'F4'],
  );
});

test('root entries and folder children keep tree order and drop hidden rows', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(
    visibleChildren(tree, null).map((r) => r.id),
    ['F1', 'F3', 'D6'],
  );
  assert.deepEqual(
    visibleChildren(tree, 'F1').map((r) => r.id),
    ['D1', 'D3'],
  );
  assert.deepEqual(visibleChildren(tree, 'F5'), []);
});

test('ancestors run root first regardless of the folder visibility values', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(
    visibleAncestors(tree, 'D5').map((r) => r.id),
    ['F3', 'F4'],
  );
  assert.deepEqual(visibleAncestors(tree, 'D6'), []);
});

test('neighbors are the published documents beside the current one under the same parent', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(Object.fromEntries(Object.entries(visibleNeighbors(tree, 'D1')).map(([k, v]) => [k, v?.id ?? null])), {
    prev: null,
    next: 'D3',
  });
  assert.deepEqual(Object.fromEntries(Object.entries(visibleNeighbors(tree, 'D3')).map(([k, v]) => [k, v?.id ?? null])), {
    prev: 'D1',
    next: null,
  });
  assert.deepEqual(visibleNeighbors(tree, 'D2'), { prev: null, next: null });
});

test('child counts take only the visible direct children and visible folder ids follow the visibility rule', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(visibleChildCounts(tree, 'F1'), { folders: 0, documents: 2 });
  assert.deepEqual(visibleChildCounts(tree, 'F3'), { folders: 1, documents: 0 });
  assert.deepEqual(visibleChildCounts(tree, 'F4'), { folders: 0, documents: 1 });
  assert.deepEqual(
    visibleFolderIds(tree).toSorted((a, b) => a.localeCompare(b)),
    ['F1', 'F3', 'F4'],
  );
});

test('descendant document ids include every document under the given folders regardless of visibility', () => {
  const tree = buildSiteTree(fixture);
  assert.deepEqual(
    descendantDocumentEntityIds(tree, ['F1']).toSorted((a, b) => a.localeCompare(b)),
    ['D1', 'D2', 'D3', 'D4'],
  );
  assert.deepEqual(descendantDocumentEntityIds(tree, ['F3', 'F5']), ['D5']);
});

test('a row whose parent is missing from the site rows is treated as a root', () => {
  const tree = buildSiteTree([row('D9', 'GONE', EntityType.DOCUMENT, EntityVisibility.PUBLIC, 'a')]);
  assert.deepEqual(
    visibleChildren(tree, null).map((r) => r.id),
    ['D9'],
  );
});

test('site tree rows query takes active non-divider entities of the given sites', () => {
  const query = buildSiteTreeRowsQuery(database, { siteIds: ['S0A'] }).toSQL();
  assert.match(query.sql, /"entities"\."site_id" in \(/);
  assert.match(query.sql, /"entities"\."state" in \(/);
  assert.match(query.sql, /"entities"\."type" <> /);
  assert.deepEqual(query.params, ['S0A', 'ACTIVE', 'DIVIDER']);
});
