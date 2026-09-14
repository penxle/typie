import assert from 'node:assert/strict';
import test from 'node:test';
import { TypieError } from '@typie/lib/errors';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import { buildSpacesBySiteQuery, normalizeSpaceLinks } from './space-core.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

test('spaces by site are active rows ordered by creation', () => {
  const query = buildSpacesBySiteQuery(database, { siteIds: ['S0A', 'S0B'] }).toSQL();
  assert.match(query.sql, /"spaces"\."site_id" in \(/);
  assert.match(query.sql, /"spaces"\."state" = /);
  assert.match(query.sql, /order by "spaces"\."created_at" asc/);
  assert.deepEqual(query.params, ['S0A', 'S0B', 'ACTIVE']);
});

test('space links keep label and url trimmed and drop empties', () => {
  assert.deepEqual(
    normalizeSpaceLinks([
      { label: ' 블로그 ', url: ' https://a.example ' },
      { label: '', url: 'https://b.example' },
    ]),
    [{ label: '블로그', url: 'https://a.example' }],
  );
});

test('space links reject urls that are not http or https', () => {
  const isLinkError = (error: unknown) => error instanceof TypieError && error.code === 'space_link_invalid' && error.status === 400;

  assert.throws(() => normalizeSpaceLinks([{ label: '블로그', url: 'javascript:alert(1)' }]), isLinkError);
  assert.throws(() => normalizeSpaceLinks([{ label: '블로그', url: 'not a url' }]), isLinkError);
  assert.deepEqual(normalizeSpaceLinks([{ label: '블로그', url: 'https://a.example' }]), [{ label: '블로그', url: 'https://a.example' }]);
});
