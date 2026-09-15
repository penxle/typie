import assert from 'node:assert/strict';
import test from 'node:test';
import { EntityVisibility, PublicationState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { drizzle } from 'drizzle-orm/postgres-js';
import * as tables from '#/db/schemas/tables.ts';
import {
  buildDuePublicationsQuery,
  buildLatestVersionGraphsQuery,
  buildLatestVersionMetadataQuery,
  buildPublicationsBySiteQuery,
  buildPublishedDocumentIdsQuery,
  buildPublishingEntityIdsBySiteQuery,
  hasUnpublishedChanges,
  mergeTags,
  normalizeTags,
  pickPublicationVersion,
  resolveBulkPublishPlan,
  resolvePublishedVisibilityBlock,
  resolvePublishTransition,
  resolveVisibilityRequestBlock,
  summarizePublicationStates,
  validateScheduledAt,
} from './publication-core.ts';
import type { Database } from '#/db/index.ts';

const database = drizzle.mock({ schema: tables }) as unknown as Database;

const heads = (s: string) => new TextEncoder().encode(s);
const snap = (over: Partial<Parameters<typeof pickPublicationVersion>[1]> = {}) => ({
  title: '제목',
  subtitle: null,
  heads: heads('h1'),
  thumbnailId: null,
  excerpt: null,
  ...over,
});

test('normalizeTags trims, drops empties, keeps first occurrence order', () => {
  assert.deepEqual(normalizeTags([' 에세이 ', '', '소설', '에세이', '  ']), ['에세이', '소설']);
});

test('normalizeTags folds decomposed hangul into the composed form so equal names collapse', () => {
  const decomposed = '에세이'.normalize('NFD');
  assert.notEqual(decomposed, '에세이');
  assert.deepEqual(normalizeTags([decomposed, '에세이']), ['에세이']);
});

test('validateScheduledAt rejects past or now and passes future', () => {
  const now = dayjs('2026-09-07T12:00:00+09:00');
  assert.equal(validateScheduledAt(null, now), null);
  assert.throws(
    () => validateScheduledAt(now, now),
    (e: unknown) => e instanceof TypieError && e.code === 'publication_scheduled_in_past',
  );
  assert.throws(() => validateScheduledAt(now.subtract(1, 'minute'), now), TypieError);
  assert.ok(validateScheduledAt(now.add(1, 'minute'), now)?.isAfter(now));
});

test('pickPublicationVersion starts at 1 and reuses when the edition is identical', () => {
  assert.deepEqual(pickPublicationVersion(null, snap()), { reuse: false, version: 1 });
  assert.deepEqual(pickPublicationVersion({ ...snap(), version: 3 }, snap()), { reuse: true, version: 3 });
});

test('pickPublicationVersion bumps when any of heads, title, subtitle, thumbnail, excerpt differ', () => {
  const latest = { ...snap(), version: 2 };
  assert.deepEqual(pickPublicationVersion(latest, snap({ heads: heads('h2') })), { reuse: false, version: 3 });
  assert.deepEqual(pickPublicationVersion(latest, snap({ title: '다른 제목' })), { reuse: false, version: 3 });
  assert.deepEqual(pickPublicationVersion(latest, snap({ subtitle: '부제' })), { reuse: false, version: 3 });
  assert.deepEqual(pickPublicationVersion(latest, snap({ thumbnailId: 'IMG0X' })), { reuse: false, version: 3 });
  assert.deepEqual(pickPublicationVersion(latest, snap({ excerpt: '미리보기' })), { reuse: false, version: 3 });
});

test('resolvePublishTransition publishes now, schedules, and rejects an already published edition', () => {
  const now = dayjs('2026-09-07T12:00:00+09:00');
  const later = now.add(2, 'hour');
  assert.deepEqual(resolvePublishTransition({ currentState: null, scheduledAt: null, now }), {
    state: PublicationState.PUBLISHED,
    publishedAt: now,
    scheduledAt: null,
  });
  assert.deepEqual(resolvePublishTransition({ currentState: PublicationState.UNPUBLISHED, scheduledAt: null, now }), {
    state: PublicationState.PUBLISHED,
    publishedAt: now,
    scheduledAt: null,
  });
  assert.deepEqual(resolvePublishTransition({ currentState: PublicationState.UNPUBLISHED, scheduledAt: later, now }), {
    state: PublicationState.SCHEDULED,
    publishedAt: null,
    scheduledAt: later,
  });
  assert.deepEqual(resolvePublishTransition({ currentState: PublicationState.SCHEDULED, scheduledAt: null, now }), {
    state: PublicationState.PUBLISHED,
    publishedAt: now,
    scheduledAt: null,
  });
  assert.throws(
    () => resolvePublishTransition({ currentState: PublicationState.PUBLISHED, scheduledAt: null, now }),
    (e: unknown) => e instanceof TypieError && e.code === 'publication_already_published',
  );
});

test('hasUnpublishedChanges compares heads, title and subtitle only', () => {
  const version = { heads: heads('h1'), title: '제목', subtitle: null };
  assert.equal(hasUnpublishedChanges(version, { heads: heads('h1'), title: '제목', subtitle: null }), false);
  assert.equal(hasUnpublishedChanges(version, { heads: heads('h2'), title: '제목', subtitle: null }), true);
  assert.equal(hasUnpublishedChanges(version, { heads: heads('h1'), title: '제목!', subtitle: null }), true);
  assert.equal(hasUnpublishedChanges(version, { heads: null, title: '제목', subtitle: null }), true);
});

test('due publications are scheduled rows at or before now on live entities and sites, locked with skip locked', () => {
  const now = dayjs('2026-09-07T12:00:00+09:00');
  const query = buildDuePublicationsQuery(database, { now }).toSQL();
  assert.match(query.sql, /"publications"\."state" = /);
  assert.match(query.sql, /"publications"\."scheduled_at" <= /);
  assert.match(query.sql, /"entities"\."state" = /);
  assert.match(query.sql, /"sites"\."state" = /);
  assert.doesNotMatch(query.sql, /"spaces"/);
  assert.match(query.sql, /for update of "publications" skip locked/);
  assert.equal(query.params[0], 'SCHEDULED');
});

test('publications by site filter state and order by published_at desc with id as the tie-break', () => {
  const query = buildPublicationsBySiteQuery(database, { siteIds: ['S0A'], states: ['PUBLISHED'] }).toSQL();
  assert.match(query.sql, /"publications"\."site_id" in \(/);
  assert.match(query.sql, /"publications"\."state" in \(/);
  assert.match(query.sql, /order by "publications"\."published_at" desc, "publications"\."id" desc/);
});

test('latest version metadata query picks one row per publication by highest version and never reads the graph', () => {
  const query = buildLatestVersionMetadataQuery(database, { publicationIds: ['PUB0A', 'PUB0B'] }).toSQL();
  assert.match(query.sql, /distinct on \("publication_versions"\."publication_id"\)/);
  assert.match(query.sql, /"publication_versions"\."version" desc/);
  assert.match(query.sql, /"title"/);
  assert.match(query.sql, /"asset_ids"/);
  assert.doesNotMatch(query.sql, /"graph"/);
});

test('latest version graph query picks one row per publication and reads the graph', () => {
  const query = buildLatestVersionGraphsQuery(database, { publicationIds: ['PUB0A', 'PUB0B'] }).toSQL();
  assert.match(query.sql, /distinct on \("publication_versions"\."publication_id"\)/);
  assert.match(query.sql, /"publication_versions"\."version" desc/);
  assert.match(query.sql, /"graph"/);
});

test('published document ids query filters the given documents by the published state', () => {
  const query = buildPublishedDocumentIdsQuery(database, { documentIds: ['DOC0A', 'DOC0B'] }).toSQL();
  assert.match(query.sql, /"publications"\."document_id" in \(/);
  assert.match(query.sql, /"publications"\."state" = /);
  assert.equal(query.params.at(-1), 'PUBLISHED');
});

test('publishing entity ids by site read the site column and take published and scheduled rows', () => {
  const query = buildPublishingEntityIdsBySiteQuery(database, { siteIds: ['S0A', 'S0B'] }).toSQL();
  assert.doesNotMatch(query.sql, /"spaces"/);
  assert.match(query.sql, /"publications"\."site_id" in \(/);
  assert.match(query.sql, /"publications"\."state" in \(/);
  assert.deepEqual(query.params, ['S0A', 'S0B', 'PUBLISHED', 'SCHEDULED']);
});

test('published visibility block names the blocking code for unlisted and private, and passes public', () => {
  assert.equal(resolvePublishedVisibilityBlock(EntityVisibility.UNLISTED), 'publication_link_share_blocked');
  assert.equal(resolvePublishedVisibilityBlock(EntityVisibility.PRIVATE), 'publication_unpublish_required');
  assert.equal(resolvePublishedVisibilityBlock(EntityVisibility.PUBLIC), null);
});

test('PUBLIC visibility is reserved for publishing and cannot be requested directly', () => {
  assert.equal(resolveVisibilityRequestBlock(EntityVisibility.PUBLIC), 'visibility_public_reserved');
  assert.equal(resolveVisibilityRequestBlock(EntityVisibility.UNLISTED), null);
  assert.equal(resolveVisibilityRequestBlock(EntityVisibility.PRIVATE), null);
});

test('mergeTags keeps existing order, appends added ones, drops removed ones and duplicates', () => {
  assert.deepEqual(mergeTags(['에세이', '소설'], ['소설', '일기'], ['에세이']), ['소설', '일기']);
  assert.deepEqual(mergeTags([], undefined, undefined), []);
});

test('resolveBulkPublishPlan republishes a published edition with merged tags', () => {
  const [step] = resolveBulkPublishPlan(
    [{ documentId: 'D1', existing: { state: PublicationState.PUBLISHED, scheduledAt: null, tags: ['a'] } }],
    { addTags: ['b'] },
  );
  assert.deepEqual(step, { kind: 'republish', documentId: 'D1', tags: ['a', 'b'] });
});

test('resolveBulkPublishPlan publishes an unpublished document right away', () => {
  const [step] = resolveBulkPublishPlan([{ documentId: 'D1', existing: null }], {});
  assert.deepEqual(step, { kind: 'publish', documentId: 'D1', tags: [], scheduledAt: null, currentState: null });
});

test('resolveBulkPublishPlan keeps the scheduled time when scheduledAt is omitted and drops it on null', () => {
  const at = dayjs('2026-09-20T10:00:00+09:00');
  const existing = { state: PublicationState.SCHEDULED, scheduledAt: at, tags: ['a'] };
  const [kept] = resolveBulkPublishPlan([{ documentId: 'D1', existing }], {});
  assert.ok(kept.kind === 'publish' && kept.scheduledAt?.isSame(at));
  const [now] = resolveBulkPublishPlan([{ documentId: 'D1', existing }], { scheduledAt: null });
  assert.ok(now.kind === 'publish' && now.scheduledAt === null && now.currentState === PublicationState.SCHEDULED);
});

test('resolveBulkPublishPlan plans a mixed selection in input order and lets removeTags win over addTags', () => {
  const at = dayjs('2026-09-20T10:00:00+09:00');
  const steps = resolveBulkPublishPlan(
    [
      { documentId: 'D1', existing: null },
      { documentId: 'D2', existing: { state: PublicationState.SCHEDULED, scheduledAt: at, tags: ['a', 'b'] } },
      { documentId: 'D3', existing: { state: PublicationState.PUBLISHED, scheduledAt: null, tags: ['a'] } },
    ],
    { addTags: ['c', 'b'], removeTags: ['a', 'c'] },
  );
  assert.deepEqual(
    steps.map((step) => [step.kind, step.documentId, step.tags]),
    [
      ['publish', 'D1', ['b']],
      ['publish', 'D2', ['b']],
      ['republish', 'D3', ['b']],
    ],
  );
});

test('summarizePublicationStates counts published, scheduled and the rest as unpublished', () => {
  assert.deepEqual(
    summarizePublicationStates([PublicationState.PUBLISHED, PublicationState.SCHEDULED, PublicationState.UNPUBLISHED, null]),
    { published: 1, scheduled: 1, unpublished: 2 },
  );
});
