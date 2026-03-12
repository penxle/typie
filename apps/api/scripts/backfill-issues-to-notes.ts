import { and, asc, desc, eq, inArray, isNull, sql } from 'drizzle-orm';
import { db, Entities, IssueEntities, Issues, NoteEntities, Notes, Sites } from '@/db';
import { TableCode } from '@/db/schemas/codes';
import { createDbId } from '@/db/schemas/id';
import { generateFractionalOrder } from '@/utils/order';

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

  const issues = await db.select().from(Issues).where(inArray(Issues.id, issueIds));
  const notes = await db.select().from(Notes).where(inArray(Notes.id, noteIds));

  const issueMap = new Map(issues.map((i) => [i.id, i]));
  const noteMap = new Map(notes.map((n) => [n.id, n]));

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

    // Sync issue_entities → note_entities
    const issueEntityRows = await db
      .select({ entityId: IssueEntities.entityId })
      .from(IssueEntities)
      .where(eq(IssueEntities.issueId, issue_id));

    if (issueEntityRows.length > 0) {
      await db
        .insert(NoteEntities)
        .values(
          issueEntityRows.map((r) => ({
            id: createDbId(TableCode.NOTE_ENTITIES),
            noteId: note_id,
            entityId: r.entityId,
          })),
        )
        .onConflictDoNothing();
    }
  }

  console.log(`  Step B-1 progress: ${i + batch.length}/${mappingRows.length} (synced: ${syncedCount})`);
}
console.log(`Step B-1 complete: ${syncedCount} notes synced from issues`);

// ---------------------------------------------------------------------------
// Step B-2: 신규 issues → notes 삽입 (매핑 없는 것)
// ---------------------------------------------------------------------------
console.log('--- Step B-2: Insert unmapped issues as notes ---');

const mappedIssueIds = new Set(mappingRows.map((r) => r.issue_id));
const allIssues = await db.select().from(Issues);
const unmappedIssues = allIssues.filter((i) => !mappedIssueIds.has(i.id));

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

// Track last order per user for fractional indexing
const userLastOrder = new Map<string, string | null>();

let insertedCount = 0;
for (let i = 0; i < unmappedIssues.length; i += BATCH_SIZE) {
  const batch = unmappedIssues.slice(i, i + BATCH_SIZE);

  for (const issue of batch) {
    const userId = siteOwnerMap.get(issue.siteId);
    if (!userId) {
      console.log(`  Skipping issue ${issue.id}: site ${issue.siteId} has no owner`);
      continue;
    }

    // Get last order for this user (cached)
    if (!userLastOrder.has(userId)) {
      const lastNote = await db
        .select({ order: Notes.order })
        .from(Notes)
        .where(and(eq(Notes.userId, userId), eq(Notes.state, 'ACTIVE')))
        .orderBy(desc(Notes.order))
        .limit(1)
        .then((rows) => rows[0]);
      userLastOrder.set(userId, lastNote?.order ?? null);
    }

    const lastOrder = userLastOrder.get(userId) ?? null;
    const order = generateFractionalOrder({ lower: lastOrder, upper: null });
    userLastOrder.set(userId, order);

    const statusMap: Record<string, string> = { OPEN: 'OPEN', IN_PROGRESS: 'OPEN', RESOLVED: 'RESOLVED', CLOSED: 'RESOLVED' };

    await db.transaction(async (tx) => {
      const [note] = await tx
        .insert(Notes)
        .values({
          userId,
          siteId: issue.siteId,
          content: issue.content,
          color: 'gray',
          order,
          status: (statusMap[issue.status] ?? 'OPEN') as 'OPEN' | 'RESOLVED',
          state: issue.state === 'ACTIVE' ? 'ACTIVE' : 'DELETED',
          createdAt: issue.createdAt,
          updatedAt: issue.updatedAt,
        })
        .returning();

      // Copy issue_entities → note_entities
      const issueEntityRows = await tx
        .select({ entityId: IssueEntities.entityId })
        .from(IssueEntities)
        .where(eq(IssueEntities.issueId, issue.id));

      if (issueEntityRows.length > 0) {
        await tx
          .insert(NoteEntities)
          .values(
            issueEntityRows.map((r) => ({
              id: createDbId(TableCode.NOTE_ENTITIES),
              noteId: note.id,
              entityId: r.entityId,
            })),
          )
          .onConflictDoNothing();
      }
    });

    insertedCount++;
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

  for (const note of batch) {
    // Try note_entities → entity.siteId first
    const noteEntity = await db
      .select({ entityId: NoteEntities.entityId })
      .from(NoteEntities)
      .where(eq(NoteEntities.noteId, note.id))
      .limit(1)
      .then((rows) => rows[0]);

    let siteId: string | undefined;

    if (noteEntity) {
      const entity = await db
        .select({ siteId: Entities.siteId })
        .from(Entities)
        .where(eq(Entities.id, noteEntity.entityId))
        .then((rows) => rows[0]);
      siteId = entity?.siteId;
    }

    if (!siteId) {
      siteId = userFirstSiteMap.get(note.userId);
    }

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
