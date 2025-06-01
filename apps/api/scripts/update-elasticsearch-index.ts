#!/usr/bin/env bun

import { eq } from 'drizzle-orm';
import { db, Entities, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';
import { elastic } from '@/search';

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

    const operations = posts.flatMap(({ id, siteId, title, subtitle, text, updatedAt }) => [
      { index: { _index: 'posts', _id: id } },
      {
        id,
        siteId,
        title,
        subtitle,
        text,
        updatedAt: updatedAt.unix(),
      },
    ]);

    if (operations.length > 0) {
      await elastic.bulk({
        operations,
      });
    }

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

    const operations = deletedPosts.map(({ id }) => ({
      delete: { _index: 'posts', _id: id },
    }));

    if (operations.length > 0) {
      await elastic.bulk({
        operations,
      });
    }

    totalProcessed += deletedPosts.length;
    offset += CHUNK_SIZE;

    if (deletedPosts.length < CHUNK_SIZE) {
      break;
    }
  }

  return totalProcessed;
};

try {
  const deletedCount = await processDeletedPostsInChunks();
  console.log(`Deleted ${deletedCount} posts from Elasticsearch index.`);

  const updatedCount = await processPostsInChunks();
  console.log(`Updated ${updatedCount} posts in Elasticsearch index.`);
} catch (err) {
  console.error('Error updating Elasticsearch index:', err);
  process.exit(1);
}

process.exit(0);
