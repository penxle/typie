import { asc, inArray, sql } from 'drizzle-orm';
import { db, Entities, IssueEntities, Issues, Notes, Sites } from '@/db';

process.env.SCRIPT = '1';

const BATCH_SIZE = 100;

// Create tracking table if not exists
await db.execute(sql`
  CREATE TABLE IF NOT EXISTS _migration_note_to_issue (
    note_id TEXT PRIMARY KEY,
    issue_id TEXT NOT NULL
  )
`);

// Load already-migrated note IDs
const migratedRows = await db.execute<{ note_id: string }>(sql`SELECT note_id FROM _migration_note_to_issue`);
const alreadyMigrated = new Set(migratedRows.map((r) => r.note_id));

const notes = await db.select().from(Notes);
const remaining = notes.filter((n) => !alreadyMigrated.has(n.id));
console.log(`Found ${notes.length} notes total, ${alreadyMigrated.size} already migrated, ${remaining.length} remaining`);

if (remaining.length === 0) {
  console.log('Nothing to migrate');
  throw new Error('Nothing to migrate');
}

// Pre-fetch entity siteIds and user first-sites in bulk
const entityIds = [...new Set(remaining.map((n) => n.entityId).filter((id): id is string => id !== null))];
const userIds = [...new Set(remaining.map((n) => n.userId))];

const entitySiteMap = new Map<string, string>();
if (entityIds.length > 0) {
  for (let i = 0; i < entityIds.length; i += BATCH_SIZE) {
    const batch = entityIds.slice(i, i + BATCH_SIZE);
    const rows = await db.select({ id: Entities.id, siteId: Entities.siteId }).from(Entities).where(inArray(Entities.id, batch));
    for (const row of rows) {
      entitySiteMap.set(row.id, row.siteId);
    }
  }
}

const userSiteMap = new Map<string, string>();
for (let i = 0; i < userIds.length; i += BATCH_SIZE) {
  const batch = userIds.slice(i, i + BATCH_SIZE);
  const rows = await db
    .select({ userId: Sites.userId, id: Sites.id, createdAt: Sites.createdAt })
    .from(Sites)
    .where(inArray(Sites.userId, batch))
    .orderBy(asc(Sites.createdAt));
  for (const row of rows) {
    if (!userSiteMap.has(row.userId)) {
      userSiteMap.set(row.userId, row.id);
    }
  }
}

let migrated = 0;
let skipped = 0;

for (let i = 0; i < remaining.length; i += BATCH_SIZE) {
  const batch = remaining.slice(i, i + BATCH_SIZE);

  const results = await Promise.allSettled(
    batch.map(async (note) => {
      const siteId = (note.entityId && entitySiteMap.get(note.entityId)) || userSiteMap.get(note.userId);

      if (!siteId) {
        console.log(`Skipping note ${note.id}: user ${note.userId} has no site`);
        return 'skipped' as const;
      }

      await db.transaction(async (tx) => {
        const [issue] = await tx
          .insert(Issues)
          .values({
            siteId,
            content: note.content,
            state: note.state === 'ACTIVE' ? 'ACTIVE' : 'DELETED',
            createdAt: note.createdAt,
            updatedAt: note.updatedAt,
          })
          .returning();

        if (note.entityId) {
          await tx.insert(IssueEntities).values({
            issueId: issue.id,
            entityId: note.entityId,
          });
        }

        await tx.execute(sql`INSERT INTO _migration_note_to_issue (note_id, issue_id) VALUES (${note.id}, ${issue.id})`);
      });

      return 'migrated' as const;
    }),
  );

  for (const result of results) {
    if (result.status === 'fulfilled') {
      if (result.value === 'migrated') migrated++;
      else skipped++;
    } else {
      console.error('Failed to migrate note:', result.reason);
    }
  }

  console.log(`Progress: ${i + batch.length}/${remaining.length}`);
}

console.log(`Migration complete: ${migrated} migrated, ${skipped} skipped`);
