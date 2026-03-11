#!/usr/bin/env node

import { eq, inArray } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, Folders } from '@/db';
import { EntityState } from '@/enums';
import { meilisearch } from '@/search';

const documentIndex = meilisearch.index('documents');
const folderIndex = meilisearch.index('folders');
const CHUNK_SIZE = 100;

const processDocumentsInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const documents = await db
      .select({
        id: Documents.id,
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

    if (documents.length === 0) {
      break;
    }

    const docs = documents.map(({ updatedAt, ...rest }) => ({
      ...rest,
      updatedAt: updatedAt.unix(),
    }));

    await documentIndex.addDocuments(docs);

    totalProcessed += documents.length;
    offset += CHUNK_SIZE;

    if (documents.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

const processDeletedDocumentsInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const deletedDocuments = await db
      .select({ id: Documents.id })
      .from(Documents)
      .innerJoin(Entities, eq(Entities.id, Documents.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .orderBy(Documents.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedDocuments.length === 0) {
      break;
    }

    const ids = deletedDocuments.map(({ id }) => id);
    await documentIndex.deleteDocuments(ids);

    totalProcessed += deletedDocuments.length;
    offset += CHUNK_SIZE;

    if (deletedDocuments.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

const processFoldersInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const folders = await db
      .select({
        id: Folders.id,
        siteId: Entities.siteId,
        name: Folders.name,
      })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(eq(Entities.state, EntityState.ACTIVE))
      .orderBy(Folders.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (folders.length === 0) {
      break;
    }

    await folderIndex.addDocuments(folders);

    totalProcessed += folders.length;
    offset += CHUNK_SIZE;

    if (folders.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

const processDeletedFoldersInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const deletedFolders = await db
      .select({ id: Folders.id })
      .from(Folders)
      .innerJoin(Entities, eq(Entities.id, Folders.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .orderBy(Folders.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedFolders.length === 0) {
      break;
    }

    const ids = deletedFolders.map(({ id }) => id);
    await folderIndex.deleteDocuments(ids);

    totalProcessed += deletedFolders.length;
    offset += CHUNK_SIZE;

    if (deletedFolders.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

try {
  await processDeletedDocumentsInChunks();
  await processDocumentsInChunks();
  await processDeletedFoldersInChunks();
  await processFoldersInChunks();
} catch {
  process.exit(1);
}

process.exit(0);
