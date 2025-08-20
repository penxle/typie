#!/usr/bin/env node

import { eq, inArray } from 'drizzle-orm';
import { Canvases, db, Entities, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';
import { meilisearch } from '@/search';

const postIndex = meilisearch.index('posts');
const canvasIndex = meilisearch.index('canvases');
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

const processCanvasesInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const canvases = await db
      .select({ id: Canvases.id, siteId: Entities.siteId, title: Canvases.title, updatedAt: Canvases.updatedAt })
      .from(Canvases)
      .innerJoin(Entities, eq(Entities.id, Canvases.entityId))
      .where(eq(Entities.state, EntityState.ACTIVE))
      .orderBy(Canvases.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (canvases.length === 0) {
      break;
    }

    const documents = canvases.map(({ updatedAt, ...rest }) => ({
      ...rest,
      updatedAt: updatedAt.unix(),
    }));

    await canvasIndex.addDocuments(documents);

    totalProcessed += canvases.length;
    offset += CHUNK_SIZE;

    if (canvases.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

const processDeletedCanvasesInChunks = async (): Promise<number> => {
  let offset = 0;
  let totalProcessed = 0;

  while (true) {
    const deletedCanvases = await db
      .select({ id: Canvases.id })
      .from(Canvases)
      .innerJoin(Entities, eq(Entities.id, Canvases.entityId))
      .where(inArray(Entities.state, [EntityState.DELETED, EntityState.PURGED]))
      .orderBy(Canvases.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedCanvases.length === 0) {
      break;
    }

    const ids = deletedCanvases.map(({ id }) => id);
    await canvasIndex.deleteDocuments(ids);

    totalProcessed += deletedCanvases.length;
    offset += CHUNK_SIZE;

    if (deletedCanvases.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

try {
  await processDeletedPostsInChunks();
  await processPostsInChunks();
  await processCanvasesInChunks();
  await processDeletedCanvasesInChunks();
} catch {
  process.exit(1);
}

process.exit(0);
