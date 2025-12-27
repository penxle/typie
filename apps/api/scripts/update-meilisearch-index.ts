#!/usr/bin/env node

import { eq, inArray } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';
import { meilisearch } from '@/search';

const postIndex = meilisearch.index('posts');
const documentIndex = meilisearch.index('documents');
const CHUNK_SIZE = 100;

const processPostsInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const posts = await db
      .select({
        id: Posts.id,
        siteId: Entities.siteId,
        title: Posts.title,
        subtitle: Posts.subtitle,
        text: PostContents.text,
        updatedAt: Posts.updatedAt,
      })
      .from(Posts)
      .innerJoin(Entities, eq(Entities.id, Posts.entityId))
      .innerJoin(PostContents, eq(PostContents.postId, Posts.id))
      .where(eq(Entities.state, EntityState.ACTIVE))
      .orderBy(Posts.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (posts.length === 0) {
      break;
    }

    const documents = posts.map(({ updatedAt, ...rest }) => ({
      ...rest,
      updatedAt: updatedAt.unix(),
    }));

    await postIndex.addDocuments(documents);

    totalProcessed += posts.length;
    offset += CHUNK_SIZE;

    if (posts.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

const processDeletedPostsInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const deletedPosts = await db
      .select({ id: Posts.id })
      .from(Posts)
      .innerJoin(Entities, eq(Entities.id, Posts.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .orderBy(Posts.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedPosts.length === 0) {
      break;
    }

    const ids = deletedPosts.map(({ id }) => id);
    await postIndex.deleteDocuments(ids);

    totalProcessed += deletedPosts.length;
    offset += CHUNK_SIZE;

    if (deletedPosts.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

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
  await processDeletedPostsInChunks();
  await processPostsInChunks();
  await processDeletedDocumentsInChunks();
  await processDocumentsInChunks();
} catch {
  process.exit(1);
}

process.exit(0);
