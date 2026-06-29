import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { redis } from '#/cache.ts';
import { db, Documents, DocumentStates, Entities, first, firstOrThrow, TableCode, validateDbId } from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { readMergedGraph } from '#/utils/changeset.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { builder } from '../builder.ts';
import { Document, DocumentState } from '../objects.ts';

/**
 * * Types
 */

DocumentState.implement({
  fields: (t) => ({
    graph: t.field({
      type: 'Binary',
      resolve: (self) => readMergedGraph(self.documentId),
    }),
    durableHeads: t.field({
      type: 'Binary',
      resolve: (self) => wasm.use((host) => host.heads(self.graph)),
    }),
    json: t.expose('json', { type: 'JSON' }),
    text: t.exposeString('text'),
    characterCount: t.exposeInt('characterCount'),
    blobSize: t.field({ type: 'BigInt', resolve: (self) => String(self.blobSize) }),
    updatedAt: t.expose('updatedAt', { type: 'DateTime' }),
  }),
});

builder.objectFields(Document, (t) => ({
  state: t.field({
    type: DocumentState,
    nullable: true,
    resolve: async (document) => db.select().from(DocumentStates).where(eq(DocumentStates.documentId, document.id)).then(first),
  }),
}));

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  pushDocumentChangesets: t.withAuth({ session: true }).fieldWithInput({
    type: builder.simpleObject('PushDocumentChangesetsPayload', {
      fields: (t) => ({
        heads: t.field({ type: 'Binary' }),
        durableHeads: t.field({ type: 'Binary' }),
      }),
    }),
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      clientId: t.input.string(),
      changesets: t.input.field({ type: 'Binary' }),
    },
    resolve: async (_, { input }, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      let opsCount: number;
      try {
        opsCount = await wasm.use((host) => host.peek_changeset_ops_count(input.changesets));
      } catch {
        throw new TypieError({ code: 'invalid_changeset_payload' });
      }

      if (opsCount > 0) {
        await redis.lpush(
          `document:changesets:pending:${input.documentId}`,
          JSON.stringify({
            userId: ctx.session.userId,
            deviceId: ctx.session.deviceId,
            changesets: input.changesets.toBase64(),
          }),
        );
      }

      // Compute heads after lpush so the merged graph reflects this push.
      // Receivers of the broadcast see heads that include the just-pushed bundle.
      const persistedState = await db
        .select({ graph: DocumentStates.graph })
        .from(DocumentStates)
        .where(eq(DocumentStates.documentId, input.documentId))
        .then(firstOrThrow);
      const graph = await readMergedGraph(input.documentId, persistedState.graph);
      const { heads, durableHeads } = await wasm.use((host) => ({
        heads: host.heads(graph),
        durableHeads: host.heads(persistedState.graph),
      }));

      if (opsCount > 0) {
        pubsub.publish('document:changesets', input.documentId, {
          target: `!${input.clientId}`,
          changesets: input.changesets.toBase64(),
          heads: heads.toBase64(),
          durableHeads: durableHeads.toBase64(),
        });

        await enqueueJob('document:changesets:collect', input.documentId);
      }

      return { heads, durableHeads };
    },
  }),

  pullDocumentChangesets: t.withAuth({ session: true }).fieldWithInput({
    type: builder.simpleObject('PullDocumentChangesetsPayload', {
      fields: (t) => ({
        changesets: t.field({ type: 'Binary' }),
        heads: t.field({ type: 'Binary' }),
        durableHeads: t.field({ type: 'Binary' }),
      }),
    }),
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      heads: t.input.field({ type: 'Binary' }),
    },
    resolve: async (_, { input }, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      const persistedState = await db
        .select({ graph: DocumentStates.graph })
        .from(DocumentStates)
        .where(eq(DocumentStates.documentId, input.documentId))
        .then(firstOrThrow);
      const graph = await readMergedGraph(input.documentId, persistedState.graph);

      const { changesets, heads, durableHeads } = await wasm.use((host) => ({
        changesets: host.missing_for(graph, input.heads),
        heads: host.heads(graph),
        durableHeads: host.heads(persistedState.graph),
      }));

      return { changesets, heads, durableHeads };
    },
  }),
}));

/**
 * * Subscriptions
 */

builder.subscriptionFields((t) => ({
  documentChangesetsUpdated: t.withAuth({ session: true }).field({
    type: builder.simpleObject('DocumentChangesetsUpdatedEvent', {
      fields: (t) => ({
        changesets: t.field({ type: 'Binary' }),
        heads: t.field({ type: 'Binary' }),
        durableHeads: t.field({ type: 'Binary' }),
      }),
    }),
    args: {
      documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      clientId: t.arg.string(),
      heads: t.arg({ type: 'Binary' }),
    },
    subscribe: async (_, args, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, args.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      const stateRow = await db
        .select({ documentId: DocumentStates.documentId })
        .from(DocumentStates)
        .where(eq(DocumentStates.documentId, args.documentId))
        .then(first);
      if (!stateRow) {
        throw new TypieError({ code: 'document_state_not_found' });
      }

      type Event = { target: string; changesets: Uint8Array; heads: Uint8Array; durableHeads: Uint8Array };

      const repeater = new Repeater<Event>(async (push, stop) => {
        const liveBuffer: Event[] = [];
        let catchupComplete = false;

        const liveStream = pubsub.subscribe('document:changesets', args.documentId);

        const livePromise = (async () => {
          for await (const event of liveStream) {
            const decoded: Event = {
              target: event.target,
              changesets: Uint8Array.fromBase64(event.changesets),
              heads: Uint8Array.fromBase64(event.heads),
              durableHeads: Uint8Array.fromBase64(event.durableHeads),
            };
            if (catchupComplete) {
              await push(decoded);
            } else {
              liveBuffer.push(decoded);
            }
          }
        })();

        const persistedState = await db
          .select({ graph: DocumentStates.graph })
          .from(DocumentStates)
          .where(eq(DocumentStates.documentId, args.documentId))
          .then(firstOrThrow);
        const graph = await readMergedGraph(args.documentId, persistedState.graph);
        const { changesets, heads, durableHeads } = await wasm.use((host) => ({
          changesets: host.missing_for(graph, args.heads),
          heads: host.heads(graph),
          durableHeads: host.heads(persistedState.graph),
        }));
        await push({ target: '*', changesets, heads, durableHeads });

        catchupComplete = true;
        for (const event of liveBuffer) {
          await push(event);
        }

        await livePromise;
        stop();
      });

      return pipe(
        repeater,
        filter(({ target }: Event) => {
          if (target === '*') return true;
          if (target.startsWith('!')) return target.slice(1) !== args.clientId;
          return target === args.clientId;
        }),
      );
    },
    resolve: ({ changesets, heads, durableHeads }) => ({ changesets, heads, durableHeads }),
  }),
}));
