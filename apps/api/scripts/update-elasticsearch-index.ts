#!/usr/bin/env node

import { EntityState } from '@typie/lib/enums';
import { count, eq, inArray, sql } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, Folders } from '#/db/index.ts';
import { elasticsearch, esIndex } from '#/search.ts';
import { decompose } from '#/utils/text.ts';

process.env.SCRIPT = '1';

const CHUNK_SIZE = 500;

const formatEta = (ms: number): string => {
  const seconds = Math.ceil(ms / 1000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = seconds % 60;
  return `${minutes}m ${remainingSeconds}s`;
};

const logProgress = (label: string, processed: number, total: number, startTime: number) => {
  const elapsed = Date.now() - startTime;
  const rate = processed / elapsed;
  const remaining = total - processed;
  const eta = rate > 0 ? formatEta(remaining / rate) : '?';
  const percent = total > 0 ? Math.round((processed / total) * 100) : 100;
  process.stdout.write(`\r${label}: ${processed}/${total} (${percent}%) ETA ${eta}  `);
};

const getAncestorEntityIdsBatch = async (entityIds: string[]): Promise<Map<string, string[]>> => {
  if (entityIds.length === 0) return new Map();

  const ids = sql.join(
    entityIds.map((id) => sql`${id}`),
    sql`,`,
  );
  const rows = await db.execute<{ source_id: string; ancestor_id: string }>(sql`
    WITH RECURSIVE ancestors AS (
      SELECT id, parent_id, id AS source_id
      FROM entities
      WHERE id IN (${ids})
      UNION ALL
      SELECT e.id, e.parent_id, a.source_id
      FROM entities e
      INNER JOIN ancestors a ON a.parent_id = e.id
    )
    SELECT source_id, id AS ancestor_id
    FROM ancestors
    WHERE id != source_id
  `);

  const map = new Map<string, string[]>();
  for (const id of entityIds) {
    map.set(id, []);
  }
  for (const row of rows) {
    map.get(row.source_id)?.push(row.ancestor_id);
  }
  return map;
};

const processDocumentsInChunks = async (total: number): Promise<number> => {
  let cursor = '';
  let totalProcessed = 0;
  const startTime = Date.now();

  while (true) {
    const documents = await db
      .select({
        id: Documents.id,
        entityId: Entities.id,
        siteId: Entities.siteId,
        title: Documents.title,
        subtitle: Documents.subtitle,
        text: DocumentContents.text,
        updatedAt: Documents.updatedAt,
      })
      .from(Documents)
      .innerJoin(Entities, eq(Entities.id, Documents.entityId))
      .innerJoin(DocumentContents, eq(DocumentContents.documentId, Documents.id))
      .where(
        cursor ? sql`${Entities.state} = ${EntityState.ACTIVE} AND ${Documents.id} > ${cursor}` : eq(Entities.state, EntityState.ACTIVE),
      )
      .orderBy(Documents.id)
      .limit(CHUNK_SIZE);

    if (documents.length === 0) break;

    const ancestorMap = await getAncestorEntityIdsBatch(documents.map((d) => d.entityId));

    const operations = [];
    for (const doc of documents) {
      operations.push(
        { index: { _index: esIndex.documents, _id: doc.id } },
        {
          site_id: doc.siteId,
          title: doc.title,
          title_decomposed: decompose(doc.title),
          subtitle: doc.subtitle,
          subtitle_decomposed: decompose(doc.subtitle),
          text: doc.text,
          ancestor_ids: ancestorMap.get(doc.entityId) ?? [],
          updated_at: doc.updatedAt,
        },
      );
    }

    await elasticsearch.bulk({ operations });

    totalProcessed += documents.length;
    logProgress('Documents indexed', totalProcessed, total, startTime);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    cursor = documents.at(-1)!.id;

    if (documents.length < CHUNK_SIZE) break;
  }

  console.log();
  return totalProcessed;
};

const processDeletedDocumentsInChunks = async (total: number): Promise<number> => {
  let cursor = '';
  let totalProcessed = 0;
  const startTime = Date.now();

  while (true) {
    const deletedDocuments = await db
      .select({ id: Documents.id })
      .from(Documents)
      .innerJoin(Entities, eq(Entities.id, Documents.entityId))
      .where(
        cursor
          ? sql`${Entities.state} IN (${EntityState.DELETED}, ${EntityState.PURGED}) AND ${Documents.id} > ${cursor}`
          : inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]),
      )
      .orderBy(Documents.id)
      .limit(CHUNK_SIZE);

    if (deletedDocuments.length === 0) break;

    const operations = deletedDocuments.flatMap((doc) => [{ delete: { _index: esIndex.documents, _id: doc.id } }]);
    await elasticsearch.bulk({ operations });

    totalProcessed += deletedDocuments.length;
    logProgress('Documents deleted', totalProcessed, total, startTime);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    cursor = deletedDocuments.at(-1)!.id;

    if (deletedDocuments.length < CHUNK_SIZE) break;
  }

  if (total > 0) console.log();
  return totalProcessed;
};

const processFoldersInChunks = async (total: number): Promise<number> => {
  let cursor = '';
  let totalProcessed = 0;
  const startTime = Date.now();

  while (true) {
    const folders = await db
      .select({
        id: Folders.id,
        entityId: Entities.id,
        siteId: Entities.siteId,
        name: Folders.name,
        createdAt: Folders.createdAt,
      })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(cursor ? sql`${Entities.state} = ${EntityState.ACTIVE} AND ${Folders.id} > ${cursor}` : eq(Entities.state, EntityState.ACTIVE))
      .orderBy(Folders.id)
      .limit(CHUNK_SIZE);

    if (folders.length === 0) break;

    const ancestorMap = await getAncestorEntityIdsBatch(folders.map((f) => f.entityId));

    const operations = [];
    for (const folder of folders) {
      operations.push(
        { index: { _index: esIndex.folders, _id: folder.id } },
        {
          site_id: folder.siteId,
          name: folder.name,
          name_decomposed: decompose(folder.name),
          ancestor_ids: ancestorMap.get(folder.entityId) ?? [],
          updated_at: folder.createdAt,
        },
      );
    }

    await elasticsearch.bulk({ operations });

    totalProcessed += folders.length;
    logProgress('Folders indexed', totalProcessed, total, startTime);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    cursor = folders.at(-1)!.id;

    if (folders.length < CHUNK_SIZE) break;
  }

  console.log();
  return totalProcessed;
};

const processDeletedFoldersInChunks = async (total: number): Promise<number> => {
  let cursor = '';
  let totalProcessed = 0;
  const startTime = Date.now();

  while (true) {
    const deletedFolders = await db
      .select({ id: Folders.id })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(
        cursor
          ? sql`${Entities.state} IN (${EntityState.DELETED}, ${EntityState.PURGED}) AND ${Folders.id} > ${cursor}`
          : inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]),
      )
      .orderBy(Folders.id)
      .limit(CHUNK_SIZE);

    if (deletedFolders.length === 0) break;

    const operations = deletedFolders.flatMap((folder) => [{ delete: { _index: esIndex.folders, _id: folder.id } }]);
    await elasticsearch.bulk({ operations });

    totalProcessed += deletedFolders.length;
    logProgress('Folders deleted', totalProcessed, total, startTime);
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    cursor = deletedFolders.at(-1)!.id;

    if (deletedFolders.length < CHUNK_SIZE) break;
  }

  if (total > 0) console.log();
  return totalProcessed;
};

try {
  console.log('Counting records...');

  const [activeDocCount, deletedDocCount, activeFolderCount, deletedFolderCount] = await Promise.all([
    db
      .select({ count: count() })
      .from(Documents)
      .innerJoin(Entities, eq(Entities.id, Documents.entityId))
      .where(eq(Entities.state, EntityState.ACTIVE))
      .then((r) => r[0].count),
    db
      .select({ count: count() })
      .from(Documents)
      .innerJoin(Entities, eq(Entities.id, Documents.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .then((r) => r[0].count),
    db
      .select({ count: count() })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(eq(Entities.state, EntityState.ACTIVE))
      .then((r) => r[0].count),
    db
      .select({ count: count() })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .then((r) => r[0].count),
  ]);

  console.log(
    `Found: ${activeDocCount} active docs, ${deletedDocCount} deleted docs, ${activeFolderCount} active folders, ${deletedFolderCount} deleted folders`,
  );

  const [deletedDocs, deletedFolders] = await Promise.all([
    processDeletedDocumentsInChunks(deletedDocCount),
    processDeletedFoldersInChunks(deletedFolderCount),
  ]);

  const indexedDocs = await processDocumentsInChunks(activeDocCount);
  const indexedFolders = await processFoldersInChunks(activeFolderCount);

  console.log(
    `Done. Documents: ${indexedDocs} indexed, ${deletedDocs} deleted. Folders: ${indexedFolders} indexed, ${deletedFolders} deleted.`,
  );
} catch (err) {
  console.error(err);
  process.exit(1);
}

process.exit(0);
