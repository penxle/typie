#!/usr/bin/env node

import { eq, inArray } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities } from '@/db';
import { EntityState } from '@/enums';
import { meilisearch } from '@/search';

const documentIndex = meilisearch.index('documents');
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

try {
  await processDeletedDocumentsInChunks();
  await processDocumentsInChunks();
} catch {
  process.exit(1);
}

process.exit(0);
