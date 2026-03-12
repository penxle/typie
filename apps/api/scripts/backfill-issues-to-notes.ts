import { and, asc, eq, inArray, isNull, sql } from 'drizzle-orm';
import { db, Entities, IssueEntities, Issues, NoteEntities, Notes, Sites } from '@/db';
import { TableCode } from '@/db/schemas/codes';
import { createDbId } from '@/db/schemas/id';
import { generateFractionalOrder } from '@/utils/order';
import type { Dayjs } from 'dayjs';

process.env.SCRIPT = '1';

const BATCH_SIZE = 100;

// ---------------------------------------------------------------------------
// Step A: notes.entity_id → note_entities 이관
// ---------------------------------------------------------------------------
console.log('--- Step A: Migrate notes.entity_id → note_entities ---');

const notesWithEntity = await db
  .select({ id: Notes.id, entityId: Notes.entityId })
  .from(Notes)
  .where(sql`${Notes.entityId} IS NOT NULL`);

let stepACount = 0;
for (let i = 0; i < notesWithEntity.length; i += BATCH_SIZE) {
  const batch = notesWithEntity.slice(i, i + BATCH_SIZE);
  await db
    .insert(NoteEntities)
    .values(
      batch.map((n) => ({
        id: createDbId(TableCode.NOTE_ENTITIES),
        noteId: n.id,
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        entityId: n.entityId!,
      })),
    )
    .onConflictDoNothing();
  stepACount += batch.length;
  console.log(`  Step A progress: ${stepACount}/${notesWithEntity.length}`);
}
// spell-checker:disable-next-line
console.log(`Step A complete: ${stepACount} note_entities upserted`);

// ---------------------------------------------------------------------------
// Step B-1: 기존 매핑된 issues → notes content 동기화
// ---------------------------------------------------------------------------
console.log('--- Step B-1: Sync mapped issues content to notes ---');

const mappingRows = await db.execute<{ note_id: string; issue_id: string }>(sql`SELECT note_id, issue_id FROM _migration_note_to_issue`);

let syncedCount = 0;
for (let i = 0; i < mappingRows.length; i += BATCH_SIZE) {
  const batch = mappingRows.slice(i, i + BATCH_SIZE);
  const issueIds = batch.map((r) => r.issue_id);
  const noteIds = batch.map((r) => r.note_id);

  const [issues, notes, allIssueEntities] = await Promise.all([
    db.select().from(Issues).where(inArray(Issues.id, issueIds)),
    db.select().from(Notes).where(inArray(Notes.id, noteIds)),
    db
      .select({ issueId: IssueEntities.issueId, entityId: IssueEntities.entityId })
      .from(IssueEntities)
      .where(inArray(IssueEntities.issueId, issueIds)),
  ]);

  const issueMap = new Map(issues.map((i) => [i.id, i]));
  const noteMap = new Map(notes.map((n) => [n.id, n]));
  const issueEntityMap = new Map<string, string[]>();
  for (const r of allIssueEntities) {
    const arr = issueEntityMap.get(r.issueId) ?? [];
    arr.push(r.entityId);
    issueEntityMap.set(r.issueId, arr);
  }

  const noteEntityValues: { id: string; noteId: string; entityId: string }[] = [];

  for (const { note_id, issue_id } of batch) {
    const issue = issueMap.get(issue_id);
    const note = noteMap.get(note_id);
    if (!issue || !note) continue;

    if (issue.updatedAt > note.updatedAt) {
      const statusMap: Record<string, string> = { OPEN: 'OPEN', IN_PROGRESS: 'OPEN', RESOLVED: 'RESOLVED', CLOSED: 'RESOLVED' };
      await db
        .update(Notes)
        .set({
          content: issue.content,
          status: (statusMap[issue.status] ?? 'OPEN') as 'OPEN' | 'RESOLVED',
          state: issue.state === 'ACTIVE' ? 'ACTIVE' : 'DELETED',
          updatedAt: issue.updatedAt,
        })
        .where(eq(Notes.id, note_id));
      syncedCount++;
    }

    const entityIds = issueEntityMap.get(issue_id) ?? [];
    for (const entityId of entityIds) {
      noteEntityValues.push({ id: createDbId(TableCode.NOTE_ENTITIES), noteId: note_id, entityId });
    }
  }

  if (noteEntityValues.length > 0) {
    await db.insert(NoteEntities).values(noteEntityValues).onConflictDoNothing();
  }

  console.log(`  Step B-1 progress: ${i + batch.length}/${mappingRows.length} (synced: ${syncedCount})`);
}
console.log(`Step B-1 complete: ${syncedCount} notes synced from issues`);

// ---------------------------------------------------------------------------
// Step B-2: 신규 issues → notes 삽입 (매핑 없는 것)
// ---------------------------------------------------------------------------
console.log('--- Step B-2: Insert unmapped issues as notes ---');

const mappedIssueIds = new Set(mappingRows.map((r) => r.issue_id));
const allUnmappedIssues = await db.select().from(Issues);
const unmappedIssues = allUnmappedIssues.filter((i) => !mappedIssueIds.has(i.id));

console.log(`  Found ${unmappedIssues.length} unmapped issues`);

// Pre-fetch site owners
const siteIds = [...new Set(unmappedIssues.map((i) => i.siteId))];
const siteOwnerMap = new Map<string, string>();
if (siteIds.length > 0) {
  for (let i = 0; i < siteIds.length; i += BATCH_SIZE) {
    const batch = siteIds.slice(i, i + BATCH_SIZE);
    const sites = await db.select({ id: Sites.id, userId: Sites.userId }).from(Sites).where(inArray(Sites.id, batch));
    for (const s of sites) {
      siteOwnerMap.set(s.id, s.userId);
    }
  }
}

// Pre-fetch all issue_entities for unmapped issues
const unmappedIssueIds = unmappedIssues.map((i) => i.id);
const allUnmappedIssueEntities = new Map<string, string[]>();
for (let i = 0; i < unmappedIssueIds.length; i += BATCH_SIZE) {
  const batch = unmappedIssueIds.slice(i, i + BATCH_SIZE);
  const rows = await db
    .select({ issueId: IssueEntities.issueId, entityId: IssueEntities.entityId })
    .from(IssueEntities)
    .where(inArray(IssueEntities.issueId, batch));
  for (const r of rows) {
    const arr = allUnmappedIssueEntities.get(r.issueId) ?? [];
    arr.push(r.entityId);
    allUnmappedIssueEntities.set(r.issueId, arr);
  }
}

// Pre-fetch last order per user
const userIdsForOrder = [...new Set(unmappedIssues.map((i) => siteOwnerMap.get(i.siteId)).filter(Boolean))] as string[];
const userLastOrder = new Map<string, string | null>();
for (let i = 0; i < userIdsForOrder.length; i += BATCH_SIZE) {
  const batch = userIdsForOrder.slice(i, i + BATCH_SIZE);
  const rows = await db
    .select({ userId: Notes.userId, order: sql<string>`max(${Notes.order})`.as('order') })
    .from(Notes)
    .where(and(inArray(Notes.userId, batch), eq(Notes.state, 'ACTIVE')))
    .groupBy(Notes.userId);
  for (const r of rows) {
    userLastOrder.set(r.userId, r.order ?? null);
  }
}

const statusMap: Record<string, string> = { OPEN: 'OPEN', IN_PROGRESS: 'OPEN', RESOLVED: 'RESOLVED', CLOSED: 'RESOLVED' };

let insertedCount = 0;
for (let i = 0; i < unmappedIssues.length; i += BATCH_SIZE) {
  const batch = unmappedIssues.slice(i, i + BATCH_SIZE);

  const noteValues: {
    id: string;
    userId: string;
    siteId: string;
    content: typeof Issues.$inferSelect.content;
    color: string;
    order: string;
    status: 'OPEN' | 'RESOLVED';
    state: 'ACTIVE' | 'DELETED';
    createdAt: Dayjs;
    updatedAt: Dayjs;
  }[] = [];
  const issueToNoteId = new Map<string, string>();

  for (const issue of batch) {
    const userId = siteOwnerMap.get(issue.siteId);
    if (!userId) {
      console.log(`  Skipping issue ${issue.id}: site ${issue.siteId} has no owner`);
      continue;
    }

    const lastOrder = userLastOrder.get(userId) ?? null;
    const order = generateFractionalOrder({ lower: lastOrder, upper: null });
    userLastOrder.set(userId, order);

    const noteId = createDbId(TableCode.NOTES);
    issueToNoteId.set(issue.id, noteId);

    noteValues.push({
      id: noteId,
      userId,
      siteId: issue.siteId,
      content: issue.content,
      color: 'gray',
      order,
      status: (statusMap[issue.status] ?? 'OPEN') as 'OPEN' | 'RESOLVED',
      state: issue.state === 'ACTIVE' ? 'ACTIVE' : 'DELETED',
      createdAt: issue.createdAt,
      updatedAt: issue.updatedAt,
    });
  }

  if (noteValues.length > 0) {
    await db.insert(Notes).values(noteValues);

    const noteEntityValues: { id: string; noteId: string; entityId: string }[] = [];
    for (const [issueId, noteId] of issueToNoteId) {
      const entityIds = allUnmappedIssueEntities.get(issueId) ?? [];
      for (const entityId of entityIds) {
        noteEntityValues.push({ id: createDbId(TableCode.NOTE_ENTITIES), noteId, entityId });
      }
    }

    if (noteEntityValues.length > 0) {
      await db.insert(NoteEntities).values(noteEntityValues).onConflictDoNothing();
    }

    insertedCount += noteValues.length;
  }

  console.log(`  Step B-2 progress: ${i + batch.length}/${unmappedIssues.length} (inserted: ${insertedCount})`);
}
console.log(`Step B-2 complete: ${insertedCount} notes inserted from unmapped issues`);

// ---------------------------------------------------------------------------
// Step C: notes.site_id 채우기
// ---------------------------------------------------------------------------
console.log('--- Step C: Backfill notes.site_id ---');

const notesWithoutSite = await db.select({ id: Notes.id, userId: Notes.userId }).from(Notes).where(isNull(Notes.siteId));

console.log(`  Found ${notesWithoutSite.length} notes without site_id`);

// Pre-fetch user → first site
const userIdsForSite = [...new Set(notesWithoutSite.map((n) => n.userId))];
const userFirstSiteMap = new Map<string, string>();
for (let i = 0; i < userIdsForSite.length; i += BATCH_SIZE) {
  const batch = userIdsForSite.slice(i, i + BATCH_SIZE);
  const sites = await db
    .select({ userId: Sites.userId, id: Sites.id })
    .from(Sites)
    .where(inArray(Sites.userId, batch))
    .orderBy(asc(Sites.createdAt));
  for (const s of sites) {
    if (!userFirstSiteMap.has(s.userId)) {
      userFirstSiteMap.set(s.userId, s.id);
    }
  }
}

let filledCount = 0;
for (let i = 0; i < notesWithoutSite.length; i += BATCH_SIZE) {
  const batch = notesWithoutSite.slice(i, i + BATCH_SIZE);
  const batchNoteIds = batch.map((n) => n.id);

  // Batch: note_entities → entity.siteId via JOIN
  const noteEntitySites = await db
    .select({ noteId: NoteEntities.noteId, siteId: Entities.siteId })
    .from(NoteEntities)
    .innerJoin(Entities, eq(NoteEntities.entityId, Entities.id))
    .where(inArray(NoteEntities.noteId, batchNoteIds));

  const noteSiteFromEntity = new Map<string, string>();
  for (const r of noteEntitySites) {
    if (!noteSiteFromEntity.has(r.noteId)) {
      noteSiteFromEntity.set(r.noteId, r.siteId);
    }
  }

  for (const note of batch) {
    const siteId = noteSiteFromEntity.get(note.id) ?? userFirstSiteMap.get(note.userId);

    if (siteId) {
      await db.update(Notes).set({ siteId }).where(eq(Notes.id, note.id));
      filledCount++;
    } else {
      console.log(`  Warning: note ${note.id} user ${note.userId} has no site`);
    }
  }

  console.log(`  Step C progress: ${i + batch.length}/${notesWithoutSite.length} (filled: ${filledCount})`);
}
console.log(`Step C complete: ${filledCount} notes got site_id`);

console.log('\n=== Backfill complete ===');
