import { findChildren } from '@tiptap/core';
import dayjs from 'dayjs';
import { and, eq, gt, lt, lte, notInArray, or, sql } from 'drizzle-orm';
import { rapidhash } from 'rapidhash-js';
import * as R from 'remeda';
import { base64 } from 'rfc4648';
import { yXmlFragmentToProseMirrorRootNode } from 'y-prosemirror';
import * as Y from 'yjs';
import { redis } from '@/cache';
import { db, Entities, first, firstOrThrow, PostAnchors, PostCharacterCountChanges, PostContents, Posts, PostVersions } from '@/db';
import { EntityState } from '@/enums';
import { schema } from '@/pm';
import { pubsub } from '@/pubsub';
import { meilisearch } from '@/search';
import { makeText } from '@/utils';
import { compressZstd, decompressZstd } from '@/utils/compression';
import { queue } from '../bullmq';
import { enqueueJob } from '../index';
import { defineCron, defineJob } from '../types';
import type { Node } from '@tiptap/pm/model';

const getCharacterCount = (text: string) => {
  return [...text.replaceAll(/\s+/g, ' ').trim()].length;
};

const getBlobSize = (node: Node) => {
  const sizes = findChildren(node, (node) => node.type.name === 'file' || node.type.name === 'image').map(
    ({ node }) => Number(node.attrs.size) || 0,
  );
  return sizes.reduce((acc, size) => acc + size, 0);
};

export const PostSyncCollectJob = defineJob('post:sync:collect', async (postId: string) => {
  const updatesAvailable = await redis.scard(`post:sync:updates:${postId}`);
  if (updatesAvailable === 0) {
    return;
  }

  let snapshotUpdated = false;

  const updates = await redis.smembers(`post:sync:updates:${postId}`);
  if (updates.length === 0) {
    return;
  }

  await db.transaction(async (tx) => {
    const hash = Number(BigInt(rapidhash(postId)) % BigInt('9223372036854775807'));
    await tx.execute(sql`SELECT pg_advisory_xact_lock(${hash})`);

    const post = await tx
      .select({
        id: Posts.id,
        update: PostContents.update,
        text: PostContents.text,
        siteId: Entities.siteId,
      })
      .from(Posts)
      .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
      .innerJoin(Entities, eq(Posts.entityId, Entities.id))
      .where(eq(Posts.id, postId))
      .then(firstOrThrow);

    const pendingUpdates = R.groupBy(
      updates.map((update) => JSON.parse(update) as { userId: string; data: string }),
      ({ userId }) => userId,
    );

    const doc = new Y.Doc({ gc: false });
    Y.applyUpdateV2(doc, post.update);

    let prevCharacterCount = getCharacterCount(post.text);

    for (const [userId, data] of Object.entries(pendingUpdates)) {
      const prevSnapshot = Y.snapshot(doc);
      const update = Y.mergeUpdatesV2(data.map(({ data }) => base64.parse(data)));

      Y.applyUpdateV2(doc, update);
      const snapshot = Y.snapshot(doc);

      if (!Y.equalSnapshots(prevSnapshot, snapshot)) {
        snapshotUpdated = true;

        const newSnapshot = Y.encodeSnapshotV2(snapshot);
        const newMetadata = {
          createdAt: dayjs.utc().toISOString(),
          contributorIds: [userId],
        };

        await tx
          .update(PostVersions)
          .set({
            latests: sql`array_append(${PostVersions.latests}, ${newSnapshot})`,
            metadata: sql`
              jsonb_set(
                ${PostVersions.metadata},
                '{latests}',
                COALESCE(${PostVersions.metadata}->'latests', '[]'::jsonb) || ${JSON.stringify(newMetadata)}::jsonb
              )
            `,
            updatedAt: dayjs(),
          })
          .where(eq(PostVersions.postId, postId));

        const fragment = doc.getXmlFragment('body');
        const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
        const body = node.toJSON();
        const text = makeText(body);

        const characterCount = getCharacterCount(text);
        const characterCountDelta = characterCount - prevCharacterCount;

        if (characterCountDelta !== 0) {
          await tx
            .insert(PostCharacterCountChanges)
            .values({
              postId,
              userId,
              bucket: dayjs().startOf('hour'),
              additions: Math.max(characterCountDelta, 0),
              deletions: Math.max(-characterCountDelta, 0),
            })
            .onConflictDoUpdate({
              target: [PostCharacterCountChanges.userId, PostCharacterCountChanges.postId, PostCharacterCountChanges.bucket],
              set: {
                additions: characterCountDelta > 0 ? sql`${PostCharacterCountChanges.additions} + ${characterCountDelta}` : undefined,
                deletions: characterCountDelta < 0 ? sql`${PostCharacterCountChanges.deletions} + ${-characterCountDelta}` : undefined,
              },
            });
        }

        prevCharacterCount = characterCount;
      }
    }

    await tx
      .update(PostContents)
      .set({
        update: Y.encodeStateAsUpdateV2(doc),
        vector: Y.encodeStateVector(doc),
      })
      .where(eq(PostContents.postId, postId));

    if (snapshotUpdated) {
      const map = doc.getMap('attrs');

      const title = (map.get('title') as string) || null;
      const subtitle = (map.get('subtitle') as string) || null;
      const maxWidth = (map.get('maxWidth') as number) ?? 800;
      const coverImageId = JSON.parse((map.get('coverImage') as string) || '{}')?.id ?? null;
      const note = (map.get('note') as string) || '';
      const anchors = (map.get('anchors') as Record<string, string | null>) ?? {};

      const fragment = doc.getXmlFragment('body');
      const node = yXmlFragmentToProseMirrorRootNode(fragment, schema);
      const body = node.toJSON();
      const text = makeText(body);

      const characterCount = getCharacterCount(text);
      const blobSize = getBlobSize(node);

      const updatedAt = dayjs();

      await tx
        .update(Posts)
        .set({
          title,
          subtitle,
          maxWidth,
          coverImageId,
          updatedAt,
        })
        .where(eq(Posts.id, postId));

      await tx
        .update(PostContents)
        .set({
          body,
          text,
          characterCount,
          blobSize,
          note,
          updatedAt,
        })
        .where(eq(PostContents.postId, postId));

      await tx.delete(PostAnchors).where(and(eq(PostAnchors.postId, postId), notInArray(PostAnchors.nodeId, Object.keys(anchors))));

      for (const [nodeId, name] of Object.entries(anchors)) {
        await tx
          .insert(PostAnchors)
          .values({ postId, nodeId, name })
          .onConflictDoUpdate({
            target: [PostAnchors.postId, PostAnchors.nodeId],
            set: { name },
          });
      }
    }
  });

  await redis.srem(`post:sync:updates:${postId}`, ...updates);

  const updatesLeft = await redis.scard(`post:sync:updates:${postId}`);
  if (updatesLeft > 0) {
    await queue.removeDeduplicationKey(`post:sync:collect:${postId}`);
    await enqueueJob('post:sync:collect', postId, {
      deduplication: {
        id: `post:sync:collect:${postId}`,
      },
    });
  }

  if (snapshotUpdated) {
    const { siteId, entityId } = await db
      .select({ siteId: Entities.siteId, entityId: Entities.id })
      .from(Posts)
      .innerJoin(Entities, eq(Posts.entityId, Entities.id))
      .where(eq(Posts.id, postId))
      .then(firstOrThrow);

    pubsub.publish('site:update', siteId, { scope: 'entity', entityId });
    pubsub.publish('site:usage:update', siteId, null);

    await enqueueJob('post:index', postId, {
      deduplication: {
        id: postId,
        ttl: 60 * 1000,
      },
    });
  }
});

export const PostIndexJob = defineJob('post:index', async (postId: string) => {
  const post = await db
    .select({
      id: Posts.id,
      state: Entities.state,
      siteId: Entities.siteId,
      title: Posts.title,
      subtitle: Posts.subtitle,
      text: PostContents.text,
    })
    .from(Posts)
    .innerJoin(PostContents, eq(Posts.id, PostContents.postId))
    .innerJoin(Entities, eq(Posts.entityId, Entities.id))
    .where(eq(Posts.id, postId))
    .then(firstOrThrow);

  if (post.state === EntityState.ACTIVE) {
    await meilisearch.index('posts').addDocuments([
      {
        id: post.id,
        siteId: post.siteId,
        title: post.title,
        subtitle: post.subtitle,
        text: post.text,
      },
    ]);
  } else {
    await meilisearch.index('posts').deleteDocument(post.id);
  }
});

export const PostCompactJob = defineJob('post:compact', async (postId: string) => {
  await db.transaction(async (tx) => {
    const hash = Number(BigInt(rapidhash(postId)) % BigInt('9223372036854775807'));
    await tx.execute(sql`SELECT pg_advisory_xact_lock(${hash})`);

    const snapshot = await tx
      .select({
        archive: PostVersions.archive,
        latests: PostVersions.latests,
        metadata: PostVersions.metadata,
      })
      .from(PostVersions)
      .where(eq(PostVersions.postId, postId))
      .then(first);

    if (!snapshot) {
      return;
    }

    if (snapshot.archive.length === 0 && snapshot.latests.length === 0) {
      await tx.update(PostContents).set({ compactedAt: dayjs() }).where(eq(PostContents.postId, postId));
      return;
    }

    const allSnapshots: {
      data: Uint8Array;
      createdAt: dayjs.Dayjs;
      contributorIds: string[];
    }[] = [];

    let archive: Buffer;
    if (snapshot.archive.length > 0) {
      archive = await decompressZstd(Buffer.from(snapshot.archive));
    } else {
      archive = Buffer.alloc(0);
    }

    for (const meta of snapshot.metadata.archive) {
      const data = archive.subarray(meta.offset, meta.offset + meta.size);
      allSnapshots.push({
        data,
        createdAt: dayjs(meta.createdAt),
        contributorIds: meta.contributorIds,
      });
    }

    for (const [i, meta] of snapshot.metadata.latests.entries()) {
      allSnapshots.push({
        data: snapshot.latests[i],
        createdAt: dayjs(meta.createdAt),
        contributorIds: meta.contributorIds,
      });
    }

    const sortedSnapshots = allSnapshots.sort((a, b) => a.createdAt.valueOf() - b.createdAt.valueOf());

    const threshold24h = dayjs.utc().subtract(24, 'hours');
    const threshold2w = dayjs.utc().subtract(2, 'weeks');
    const windowedSnapshots = new Map<
      string,
      {
        createdAt: dayjs.Dayjs;
        contributorIds: Set<string>;
        snapshot: Uint8Array;
      }
    >();

    for (const snapshot of sortedSnapshots) {
      const window = snapshot.createdAt.isAfter(threshold24h)
        ? snapshot.createdAt.toISOString()
        : snapshot.createdAt.isAfter(threshold2w)
          ? snapshot.createdAt.startOf('minute').toISOString()
          : snapshot.createdAt.startOf('hour').toISOString();

      const existing = windowedSnapshots.get(window);
      if (existing) {
        existing.snapshot = snapshot.data;
        existing.createdAt = snapshot.createdAt;
        existing.contributorIds = new Set([...existing.contributorIds, ...snapshot.contributorIds]);
      } else {
        windowedSnapshots.set(window, {
          createdAt: snapshot.createdAt,
          contributorIds: new Set(snapshot.contributorIds),
          snapshot: snapshot.data,
        });
      }
    }

    const content = await tx
      .select({ update: PostContents.update })
      .from(PostContents)
      .where(eq(PostContents.postId, postId))
      .then(firstOrThrow);

    const oldDoc = new Y.Doc({ gc: false });
    Y.applyUpdateV2(oldDoc, content.update);

    const sortedWindows = [...windowedSnapshots.entries()].sort((a, b) => a[1].createdAt.valueOf() - b[1].createdAt.valueOf());

    const compactedSnapshots: {
      data: Uint8Array;
      createdAt: string;
      contributorIds: string[];
    }[] = [];

    const newDoc = new Y.Doc({ gc: false });

    for (const [i, [, window]] of sortedWindows.entries()) {
      let snapshotDoc;
      try {
        snapshotDoc = Y.createDocFromSnapshot(oldDoc, Y.decodeSnapshotV2(window.snapshot));
      } catch {
        continue;
      }

      if (i === 0) {
        Y.applyUpdateV2(newDoc, Y.encodeStateAsUpdateV2(snapshotDoc));
      } else {
        const newStateVector = Y.encodeStateVector(newDoc);
        const snapshotStateVector = Y.encodeStateVector(snapshotDoc);

        const missingUpdate = Y.encodeStateAsUpdateV2(newDoc, snapshotStateVector);
        const undoManager = new Y.UndoManager(snapshotDoc, { trackedOrigins: new Set(['snapshot']) });
        Y.applyUpdateV2(snapshotDoc, missingUpdate, 'snapshot');
        undoManager.undo();

        const revertUpdate = Y.encodeStateAsUpdateV2(snapshotDoc, newStateVector);
        Y.applyUpdateV2(newDoc, revertUpdate);
      }

      compactedSnapshots.push({
        data: Y.encodeSnapshotV2(Y.snapshot(newDoc)),
        createdAt: window.createdAt.utc().toISOString(),
        contributorIds: [...window.contributorIds],
      });
    }

    const newHistoryParts: Uint8Array[] = [];
    const newHistoryMeta: {
      createdAt: string;
      contributorIds: string[];
      offset: number;
      size: number;
    }[] = [];

    let offset = 0;
    for (const snapshot of compactedSnapshots) {
      newHistoryParts.push(snapshot.data);
      newHistoryMeta.push({
        createdAt: snapshot.createdAt,
        contributorIds: snapshot.contributorIds,
        offset,
        size: snapshot.data.length,
      });

      offset += snapshot.data.length;
    }

    const newArchive = await compressZstd(Buffer.concat(newHistoryParts));

    await tx
      .update(PostVersions)
      .set({
        archive: newArchive,
        latests: [],
        metadata: {
          latests: [],
          archive: newHistoryMeta,
        },
        updatedAt: dayjs(),
      })
      .where(eq(PostVersions.postId, postId));

    await tx
      .update(PostContents)
      .set({
        update: Y.encodeStateAsUpdateV2(newDoc),
        vector: Y.encodeStateVector(newDoc),
        compactedAt: dayjs(),
      })
      .where(eq(PostContents.postId, postId));
  });
});

export const PostCompactScanCron = defineCron('post:compact:scan', '0 * * * *', async () => {
  const now = dayjs();

  const threshold1h = now.subtract(1, 'hour');
  const threshold6h = now.subtract(6, 'hours');
  const threshold24h = now.subtract(24, 'hours');

  const threshold1d = now.subtract(1, 'day');
  const threshold2d = now.subtract(2, 'days');
  const threshold14d = now.subtract(14, 'days');

  const posts = await db
    .select({ postId: PostContents.postId })
    .from(PostContents)
    .where(
      or(
        and(lte(PostContents.updatedAt, threshold1h), gt(PostContents.updatedAt, threshold24h), lt(PostContents.compactedAt, threshold6h)),
        and(lte(PostContents.updatedAt, threshold1d), gt(PostContents.updatedAt, threshold14d), lt(PostContents.compactedAt, threshold2d)),
      ),
    );

  await Promise.all(
    posts.map(({ postId }) =>
      enqueueJob('post:compact', postId, {
        delay: Math.random() * 50 * 60 * 1000,
        priority: 1,
      }),
    ),
  );
});
