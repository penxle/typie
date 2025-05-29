#!/usr/bin/env bun

import { eq } from 'drizzle-orm';
import { db, Entities, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';
import { meili } from '@/search';

const index = meili.index('posts');
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

    await index.addDocuments(documents);

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
      .where(eq(Entities.state, EntityState.DELETED))
      .orderBy(Posts.id)
      .limit(CHUNK_SIZE)
      .offset(offset);

    if (deletedPosts.length === 0) {
      break;
    }

    const ids = deletedPosts.map(({ id }) => id);
    await index.deleteDocuments(ids);

    totalProcessed += deletedPosts.length;
    offset += CHUNK_SIZE;

    if (deletedPosts.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

try {
  await processDeletedPostsInChunks();
  await processPostsInChunks();
} catch {
  process.exit(1);
}

process.exit(0);
