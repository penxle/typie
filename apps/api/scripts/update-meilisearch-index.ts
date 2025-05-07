#!/usr/bin/env node

import { eq } from 'drizzle-orm';
import { db, Entities, PostContents, Posts } from '@/db';
import { EntityState } from '@/enums';
import { meili } from '@/search';

const index = meili.index('posts');

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
  .where(eq(Entities.state, EntityState.ACTIVE));

const deletedPosts = await db
  .select({ id: Posts.id })
  .from(Posts)
  .innerJoin(Entities, eq(Entities.id, Posts.entityId))
  .where(eq(Entities.state, EntityState.DELETED));

await index.deleteDocuments(deletedPosts.map(({ id }) => id));
await index.addDocuments(posts.map(({ updatedAt, ...rest }) => ({ ...rest, updatedAt: updatedAt.unix() })));

console.log('Indexed posts:', posts.length);

process.exit(0);
