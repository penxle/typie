import { and, eq } from 'drizzle-orm';
import * as Y from 'yjs';
import { Entities, first, firstOrThrow, Folders, PostContents, Posts, PostSnapshotContributors, PostSnapshots, Sites } from '@/db';
import { EntityState, EntityType } from '@/enums';
import { compressZstd } from './compression';
import { generatePermalink, generateSlug, makeYDoc } from './entity';
import type { Transaction } from '@/db';

type CreateSiteParams = {
  userId: string;
  name: string;
  slug: string;
  tx: Transaction;
};

export const createSite = async ({ userId, name, slug, tx }: CreateSiteParams) => {
  const site = await tx
    .insert(Sites)
    .values({
      userId,
      slug,
      name,
    })
    .returning({
      id: Sites.id,
    })
    .then(firstOrThrow);

  const templateSite = await tx
    .select({
      id: Sites.id,
    })
    .from(Sites)
    .where(eq(Sites.id, 'S0TEMPLATE'))
    .then(first);

  if (templateSite) {
    const templateFolders = await tx
      .select({
        id: Folders.id,
        name: Folders.name,
        entity: {
          id: Entities.id,
          depth: Entities.depth,
          parentId: Entities.parentId,
          order: Entities.order,
        },
      })
      .from(Folders)
      .innerJoin(Entities, eq(Folders.entityId, Entities.id))
      .where(and(eq(Entities.siteId, templateSite.id), eq(Entities.state, EntityState.ACTIVE)))
      .orderBy(Entities.depth);

    const folderEntityIdMap = new Map<string, string>();

    for (const folder of templateFolders) {
      const entity = await tx
        .insert(Entities)
        .values({
          userId,
          siteId: site.id,
          slug: generateSlug(),
          permalink: generatePermalink(),
          type: EntityType.FOLDER,
          depth: folder.entity.depth,
          parentId: folder.entity.parentId ? folderEntityIdMap.get(folder.entity.parentId) : null,
          order: folder.entity.order,
        })
        .returning({
          id: Entities.id,
        })
        .then(firstOrThrow);

      folderEntityIdMap.set(folder.entity.id, entity.id);

      await tx.insert(Folders).values({
        entityId: entity.id,
        name: folder.name,
      });
    }

    const templatePosts = await tx
      .select({
        id: Posts.id,
        title: Posts.title,
        subtitle: Posts.subtitle,
        maxWidth: Posts.maxWidth,
        content: {
          body: PostContents.body,
          text: PostContents.text,
          characterCount: PostContents.characterCount,
          blobSize: PostContents.blobSize,
        },
        entity: {
          depth: Entities.depth,
          parentId: Entities.parentId,
          order: Entities.order,
        },
      })
      .from(Posts)
      .innerJoin(Entities, eq(Posts.entityId, Entities.id))
      .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
      .where(and(eq(Entities.siteId, templateSite.id), eq(Entities.state, EntityState.ACTIVE)));

    for (const post of templatePosts) {
      const doc = makeYDoc({
        title: post.title,
        subtitle: post.subtitle,
        body: post.content.body,
        maxWidth: post.maxWidth,
      });

      const snapshot = Y.snapshot(doc);

      const newEntity = await tx
        .insert(Entities)
        .values({
          userId,
          siteId: site.id,
          parentId: post.entity.parentId ? folderEntityIdMap.get(post.entity.parentId) : null,
          slug: generateSlug(),
          permalink: generatePermalink(),
          type: EntityType.POST,
          order: post.entity.order,
          depth: post.entity.depth,
        })
        .returning({ id: Entities.id })
        .then(firstOrThrow);

      const newPost = await tx
        .insert(Posts)
        .values({
          entityId: newEntity.id,
          title: post.title,
          subtitle: post.subtitle,
          maxWidth: post.maxWidth,
        })
        .returning()
        .then(firstOrThrow);

      await tx.insert(PostContents).values({
        postId: newPost.id,
        body: post.content.body,
        text: post.content.text,
        characterCount: post.content.characterCount,
        blobSize: post.content.blobSize,
        update: Y.encodeStateAsUpdateV2(doc),
        vector: Y.encodeStateVector(doc),
      });

      const snapshotData = Y.encodeSnapshotV2(snapshot);
      const compressedSnapshot = await compressZstd(snapshotData);

      const postSnapshot = await tx
        .insert(PostSnapshots)
        .values({
          postId: newPost.id,
          snapshot: compressedSnapshot,
        })
        .returning({ id: PostSnapshots.id })
        .then(firstOrThrow);

      await tx.insert(PostSnapshotContributors).values({
        snapshotId: postSnapshot.id,
        userId,
      });
    }
  }
};
