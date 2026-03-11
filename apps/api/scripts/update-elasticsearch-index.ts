#!/usr/bin/env node

import { count, eq, inArray } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, Folders } from '@/db';
import { EntityState } from '@/enums';
import { elasticsearch, esIndex } from '@/search';
import { getAncestorEntityIds } from '@/utils/entity';
import { decompose } from '@/utils/text';

process.env.SCRIPT = '1';

const CHUNK_SIZE = 100;

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

const processDocumentsInChunks = async (total: number): Promise<number> => {
  let offset = 0;
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
      .where(eq(Entities.state, EntityState.ACTIVE))
      .orderBy(Documents.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (documents.length === 0) break;

    const operations = [];
    for (const doc of documents) {
      const ancestorIds = await getAncestorEntityIds(doc.entityId);
      operations.push(
        { index: { _index: esIndex.documents, _id: doc.id } },
        {
          site_id: doc.siteId,
          title: doc.title,
          title_decomposed: decompose(doc.title),
          subtitle: doc.subtitle,
          subtitle_decomposed: decompose(doc.subtitle),
          text: doc.text,
          ancestor_ids: ancestorIds,
          updated_at: doc.updatedAt,
        },
      );
    }

    await elasticsearch.bulk({ operations });

    totalProcessed += documents.length;
    logProgress('Documents indexed', totalProcessed, total, startTime);
    offset += CHUNK_SIZE;

    if (documents.length < CHUNK_SIZE) break;
  }

  console.log();
  return totalProcessed;
};

const processDeletedDocumentsInChunks = async (total: number): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;
  const startTime = Date.now();

  while (true) {
    const deletedDocuments = await db
      .select({ id: Documents.id })
      .from(Documents)
      .innerJoin(Entities, eq(Entities.id, Documents.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .orderBy(Documents.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedDocuments.length === 0) break;

    const operations = deletedDocuments.flatMap((doc) => [{ delete: { _index: esIndex.documents, _id: doc.id } }]);
    await elasticsearch.bulk({ operations });

    totalProcessed += deletedDocuments.length;
    logProgress('Documents deleted', totalProcessed, total, startTime);
    offset += CHUNK_SIZE;

    if (deletedDocuments.length < CHUNK_SIZE) break;
  }

  if (total > 0) console.log();
  return totalProcessed;
};

const processFoldersInChunks = async (total: number): Promise<number> => {
  let offset = 0;
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
      .where(eq(Entities.state, EntityState.ACTIVE))
      .orderBy(Folders.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (folders.length === 0) break;

    const operations = [];
    for (const folder of folders) {
      const ancestorIds = await getAncestorEntityIds(folder.entityId);
      operations.push(
        { index: { _index: esIndex.folders, _id: folder.id } },
        {
          site_id: folder.siteId,
          name: folder.name,
          name_decomposed: decompose(folder.name),
          ancestor_ids: ancestorIds,
          updated_at: folder.createdAt,
        },
      );
    }

    await elasticsearch.bulk({ operations });

    totalProcessed += folders.length;
    logProgress('Folders indexed', totalProcessed, total, startTime);
    offset += CHUNK_SIZE;

    if (folders.length < CHUNK_SIZE) break;
  }

  console.log();
  return totalProcessed;
};

const processDeletedFoldersInChunks = async (total: number): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;
  const startTime = Date.now();

  while (true) {
    const deletedFolders = await db
      .select({ id: Folders.id })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .orderBy(Folders.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedFolders.length === 0) break;

    const operations = deletedFolders.flatMap((folder) => [{ delete: { _index: esIndex.folders, _id: folder.id } }]);
    await elasticsearch.bulk({ operations });

    totalProcessed += deletedFolders.length;
    logProgress('Folders deleted', totalProcessed, total, startTime);
    offset += CHUNK_SIZE;

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

  const deletedDocs = await processDeletedDocumentsInChunks(deletedDocCount);
  const indexedDocs = await processDocumentsInChunks(activeDocCount);
  const deletedFolders = await processDeletedFoldersInChunks(deletedFolderCount);
  const indexedFolders = await processFoldersInChunks(activeFolderCount);

  console.log(
    `Done. Documents: ${indexedDocs} indexed, ${deletedDocs} deleted. Folders: ${indexedFolders} indexed, ${deletedFolders} deleted.`,
  );
} catch (err) {
  console.error(err);
  process.exit(1);
}

process.exit(0);
