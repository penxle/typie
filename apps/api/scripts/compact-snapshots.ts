#!/usr/bin/env tsx

import dayjs from 'dayjs';
import { and, lt } from 'drizzle-orm';
import { db, PostContents } from '@/db';
import { enqueueJob } from '@/mq';

process.env.SCRIPT = 'true';

console.log('Starting snapshot compaction...');

const threshold = dayjs().subtract(24, 'hours');

const posts = await db
  .select({ postId: PostContents.postId, updatedAt: PostContents.updatedAt })
  .from(PostContents)
  .where(and(lt(PostContents.updatedAt, threshold), lt(PostContents.compactedAt, PostContents.updatedAt)));

console.log(`Found ${posts.length} posts to compact`);

const batchSize = 100;
for (let i = 0; i < posts.length; i += batchSize) {
  const batch = posts.slice(i, i + batchSize);

  const promises = batch.map((post) =>
    enqueueJob('post:compact', post.postId, {
      delay: Math.random() * 60 * 60 * 1000,
      priority: 1,
    }),
  );

  await Promise.all(promises);

  console.log(`Enqueued batch ${Math.floor(i / batchSize) + 1}/${Math.ceil(posts.length / batchSize)}`);
}

console.log('Migration script completed. Jobs have been queued.');

process.exit(0);
