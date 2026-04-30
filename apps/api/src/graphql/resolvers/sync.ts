import { DocumentConflictKind } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import dayjs from 'dayjs';
import { and, asc, eq, gt, inArray } from 'drizzle-orm';
import { Repeater } from 'graphql-yoga';
import {
  db,
  DocumentCommits,
  DocumentConflictBranches,
  DocumentConflictResolutions,
  DocumentConflicts,
  DocumentObjects,
  Documents,
  Entities,
  first,
  firstOrThrow,
  TableCode,
  validateDbId,
} from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { walkReachableHashes, walkReachableObjects } from '#/utils/sync.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { builder } from '../builder.ts';
import {
  Document,
  DocumentCommit,
  DocumentConflict,
  DocumentConflictBranch,
  DocumentConflictResolution,
  DocumentObject,
  User,
  UserDevice,
} from '../objects.ts';

/**
 * * Types
 */

DocumentCommit.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    hash: t.exposeString('hash'),

    parent: t.expose('parentId', { type: DocumentCommit, nullable: true }),
    secondParent: t.expose('secondParentId', { type: DocumentCommit, nullable: true }),

    rootObject: t.expose('rootObjectId', { type: DocumentObject }),
    objects: t.field({
      type: [DocumentObject],
      resolve: async (commit) => walkReachableObjects(db, commit.rootObjectId),
    }),

    device: t.expose('deviceId', { type: UserDevice, nullable: true }),
    user: t.expose('userId', { type: User, nullable: true }),

    meta: t.expose('meta', { type: 'JSON', nullable: true }),

    committedAt: t.expose('committedAt', { type: 'DateTime' }),
    pushedAt: t.expose('pushedAt', { type: 'DateTime' }),

    conflicts: t.field({
      type: [DocumentConflict],
      resolve: async (commit) => db.select().from(DocumentConflicts).where(eq(DocumentConflicts.mergeCommitId, commit.id)),
    }),
  }),
});

DocumentConflict.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    kind: t.expose('kind', { type: DocumentConflictKind }),
    target: t.expose('target', { type: 'JSON' }),
    baseValue: t.expose('baseValue', { type: 'JSON', nullable: true }),
    mergeCommit: t.expose('mergeCommitId', { type: DocumentCommit }),

    branches: t.field({
      type: [DocumentConflictBranch],
      resolve: async (c) => db.select().from(DocumentConflictBranches).where(eq(DocumentConflictBranches.conflictId, c.id)),
    }),

    autoResolvedBranch: t.expose('autoResolvedBranchId', { type: DocumentConflictBranch, nullable: true }),

    resolution: t.field({
      type: DocumentConflictResolution,
      nullable: true,
      resolve: async (c) =>
        db.select().from(DocumentConflictResolutions).where(eq(DocumentConflictResolutions.conflictId, c.id)).then(first),
    }),

    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

DocumentConflictBranch.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    commit: t.expose('commitId', { type: DocumentCommit }),
    value: t.expose('value', { type: 'JSON' }),
  }),
});

DocumentConflictResolution.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    value: t.expose('value', { type: 'JSON' }),
    commit: t.expose('commitId', { type: DocumentCommit }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

DocumentObject.implement({
  fields: (t) => ({
    id: t.exposeID('id'),
    hash: t.exposeString('hash'),
    content: t.expose('content', { type: 'JSON' }),
    createdAt: t.expose('createdAt', { type: 'DateTime' }),
  }),
});

builder.objectFields(Document, (t) => ({
  head: t.field({
    type: DocumentCommit,
    nullable: true,
    resolve: async (document) => {
      if (!document.headCommitId) return null;
      return db.select().from(DocumentCommits).where(eq(DocumentCommits.id, document.headCommitId)).then(firstOrThrow);
    },
  }),
  objects: t.field({
    type: [DocumentObject],
    args: { hashes: t.arg.stringList() },
    resolve: async (_, { hashes }) => {
      return db.select().from(DocumentObjects).where(inArray(DocumentObjects.hash, hashes));
    },
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  pushDocumentCommits: t.withAuth({ session: true }).fieldWithInput({
    type: [DocumentCommit],
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      commits: t.input.field({
        type: [
          builder.inputType('ClientCommitInput', {
            fields: (t) => ({
              commitHash: t.string(),
              parentCommitHash: t.string(),
              rootObjectHash: t.string(),
              steps: t.field({ type: 'JSON', required: false }),
              meta: t.field({ type: 'JSON', required: false }),
              committedAt: t.field({ type: 'DateTime' }),
            }),
          }),
        ],
      }),
      objects: t.input.field({
        type: [
          builder.inputType('DocumentObjectInput', {
            fields: (t) => ({
              hash: t.string(),
              content: t.field({ type: 'JSON' }),
            }),
          }),
        ],
      }),
    },
    resolve: async (_, { input }, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      for (const o of input.objects) {
        const computed = await wasm.hash_object_content(o.content);
        if (computed !== o.hash) {
          throw new TypieError({ code: 'object_hash_mismatch' });
        }
      }

      for (const c of input.commits) {
        const computed = await wasm.hash_commit_content({
          parent_hash: c.parentCommitHash,
          object_hash: c.rootObjectHash,
        });
        if (computed !== c.commitHash) {
          throw new TypieError({ code: 'commit_hash_mismatch' });
        }
      }

      const result = await db.transaction(async (tx) => {
        let baseHashes = new Set<string>();
        if (input.commits.length > 0) {
          const baseParent = await tx
            .select({ rootObjectId: DocumentCommits.rootObjectId })
            .from(DocumentCommits)
            .where(and(eq(DocumentCommits.documentId, input.documentId), eq(DocumentCommits.hash, input.commits[0].parentCommitHash)))
            .then(first);
          if (!baseParent) {
            throw new TypieError({ code: 'invalid_parent_commit' });
          }
          baseHashes = await walkReachableHashes(tx, baseParent.rootObjectId);
        }

        const validHashes = new Set<string>([...baseHashes, ...input.objects.map((o) => o.hash)]);

        for (const c of input.commits) {
          if (!validHashes.has(c.rootObjectHash)) {
            throw new TypieError({ code: 'object_not_authorized' });
          }
        }

        for (const o of input.objects) {
          const children = (o.content as { children?: { hash: string }[] } | null)?.children ?? [];
          for (const child of children) {
            if (!validHashes.has(child.hash)) {
              throw new TypieError({ code: 'object_not_authorized' });
            }
          }
        }

        if (input.objects.length > 0) {
          await tx
            .insert(DocumentObjects)
            .values(input.objects.map((o) => ({ hash: o.hash, content: o.content })))
            .onConflictDoNothing({ target: DocumentObjects.hash });
        }

        if (input.commits.length > 0) {
          const referencedHashes = [...new Set(input.commits.map((c) => c.rootObjectHash))];
          const objectRows = await tx
            .select({ id: DocumentObjects.id, hash: DocumentObjects.hash })
            .from(DocumentObjects)
            .where(inArray(DocumentObjects.hash, referencedHashes));
          const hashToObjectId = new Map(objectRows.map((o) => [o.hash, o.id]));

          const parentCommitHashes = [...new Set(input.commits.map((c) => c.parentCommitHash))];
          const parentRows = await tx
            .select({ id: DocumentCommits.id, hash: DocumentCommits.hash })
            .from(DocumentCommits)
            .where(and(eq(DocumentCommits.documentId, input.documentId), inArray(DocumentCommits.hash, parentCommitHashes)));
          const hashToInternalId = new Map(parentRows.map((r) => [r.hash, r.id]));

          for (const c of input.commits) {
            const parentDbId = hashToInternalId.get(c.parentCommitHash);
            if (!parentDbId) {
              throw new TypieError({ code: 'invalid_parent_commit' });
            }

            const objId = hashToObjectId.get(c.rootObjectHash);
            if (!objId) {
              throw new TypieError({ code: 'missing_root_object' });
            }

            const inserted = await tx
              .insert(DocumentCommits)
              .values({
                hash: c.commitHash,
                documentId: input.documentId,
                parentId: parentDbId,
                rootObjectId: objId,
                steps: c.steps ?? null,
                meta: c.meta ?? null,
                deviceId: ctx.session.deviceId,
                userId: ctx.session.userId,
                committedAt: c.committedAt,
              })
              .onConflictDoNothing({ target: [DocumentCommits.documentId, DocumentCommits.hash] })
              .returning({ id: DocumentCommits.id })
              .then(first);

            let insertedId = inserted?.id;
            if (!insertedId) {
              const existing = await tx
                .select({ id: DocumentCommits.id })
                .from(DocumentCommits)
                .where(and(eq(DocumentCommits.documentId, input.documentId), eq(DocumentCommits.hash, c.commitHash)))
                .then(firstOrThrow);
              insertedId = existing.id;
            }

            hashToInternalId.set(c.commitHash, insertedId);
          }
        }

        await tx.update(Documents).set({ dirtyAt: dayjs() }).where(eq(Documents.id, input.documentId));

        const rows = await tx
          .select()
          .from(DocumentCommits)
          .where(
            and(
              eq(DocumentCommits.documentId, input.documentId),
              inArray(
                DocumentCommits.hash,
                input.commits.map((c) => c.commitHash),
              ),
            ),
          );
        const byHash = new Map(rows.map((r) => [r.hash, r]));
        return input.commits.map((c) => {
          const row = byHash.get(c.commitHash);
          if (!row) throw new Error('commit not persisted after insert');
          return row;
        });
      });

      await enqueueJob('document:advance-head', input.documentId);

      return result;
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  documentCommitsUpdated: t.withAuth({ session: true }).field({
    type: builder.simpleObject('DocumentCommitsUpdatedEvent', {
      fields: (t) => ({
        commits: t.field({ type: [DocumentCommit] }),
        objects: t.field({ type: [DocumentObject] }),
      }),
    }),
    args: {
      documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      sinceCommitHash: t.arg.string(),
    },
    subscribe: async (_, args, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, args.documentId))
        .then(firstOrThrow);
      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      const since = await db
        .select()
        .from(DocumentCommits)
        .where(and(eq(DocumentCommits.documentId, args.documentId), eq(DocumentCommits.hash, args.sinceCommitHash)))
        .then(first);
      if (!since) throw new TypieError({ code: 'commit_not_found' });

      type Event = { commitIds: string[]; objectIds: string[] };

      const repeater = Repeater.merge([
        pubsub.subscribe('document:commits', args.documentId),
        new Repeater<Event>(async (push, stop) => {
          const catchUpPayload = await db.transaction(async (tx) => {
            const catchUpCommits = await tx
              .select({ id: DocumentCommits.id })
              .from(DocumentCommits)
              .where(and(eq(DocumentCommits.documentId, args.documentId), gt(DocumentCommits.sequence, since.sequence)))
              .orderBy(asc(DocumentCommits.sequence));

            const document = await tx
              .select({ headCommitId: Documents.headCommitId })
              .from(Documents)
              .where(eq(Documents.id, args.documentId))
              .then(firstOrThrow);

            let catchUpObjectIds: { id: string }[] = [];
            if (document.headCommitId && document.headCommitId !== since.id) {
              const sinceHashes = await walkReachableHashes(tx, since.rootObjectId);
              const headRow = await tx
                .select({ rootObjectId: DocumentCommits.rootObjectId })
                .from(DocumentCommits)
                .where(eq(DocumentCommits.id, document.headCommitId))
                .then(firstOrThrow);
              const headHashes = await walkReachableHashes(tx, headRow.rootObjectId);
              const newHashes = [...headHashes].filter((h) => !sinceHashes.has(h));
              if (newHashes.length > 0) {
                catchUpObjectIds = await tx
                  .select({ id: DocumentObjects.id })
                  .from(DocumentObjects)
                  .where(inArray(DocumentObjects.hash, newHashes));
              }
            }

            return {
              commitIds: catchUpCommits.map((c) => c.id),
              objectIds: catchUpObjectIds.map((o) => o.id),
            };
          });

          await push(catchUpPayload);
          stop();
        }),
      ]);

      return repeater;
    },
    resolve: async (event) => {
      const commits =
        event.commitIds.length > 0
          ? await db
              .select()
              .from(DocumentCommits)
              .where(inArray(DocumentCommits.id, event.commitIds))
              .orderBy(asc(DocumentCommits.sequence))
          : [];
      const objects =
        event.objectIds.length > 0 ? await db.select().from(DocumentObjects).where(inArray(DocumentObjects.id, event.objectIds)) : [];
      return { commits, objects };
    },
  }),
}));
