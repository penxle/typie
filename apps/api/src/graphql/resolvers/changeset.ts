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
      let graph: Uint8Array;
      try {
        graph = await readMergedGraph(input.documentId);
        opsCount = await wasm.use((host) => {
          const count = host.peek_changeset_ops_count(input.changesets);
          if (count > 0) {
            const candidate = host.apply(graph, input.changesets);
            host.verify_plain(host.to_plain(candidate));
          }
          return count;
        });
      } catch {
        await enqueueJob('document:changesets:collect', input.documentId);
        throw new TypieError({ code: 'invalid_changeset_payload', status: 400 });
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
      graph = await readMergedGraph(input.documentId);
      const heads = await wasm.use((host) => host.heads(graph));

      if (opsCount > 0) {
        pubsub.publish('document:changesets', input.documentId, {
          target: `!${input.clientId}`,
          changesets: input.changesets.toBase64(),
          heads: heads.toBase64(),
        });

        await enqueueJob('document:changesets:collect', input.documentId);
      }

      return { heads };
    },
  }),

  pullDocumentChangesets: t.withAuth({ session: true }).fieldWithInput({
    type: builder.simpleObject('PullDocumentChangesetsPayload', {
      fields: (t) => ({
        changesets: t.field({ type: 'Binary' }),
        heads: t.field({ type: 'Binary' }),
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

      const graph = await readMergedGraph(input.documentId);

      const { changesets, heads } = await wasm.use((host) => ({
        changesets: host.missing_for(graph, input.heads),
        heads: host.heads(graph),
      }));

      return { changesets, heads };
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

      type Event = { target: string; changesets: Uint8Array; heads: Uint8Array };

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
            };
            if (catchupComplete) {
              await push(decoded);
            } else {
              liveBuffer.push(decoded);
            }
          }
        })();

        const graph = await readMergedGraph(args.documentId);
        const { changesets, heads } = await wasm.use((host) => ({
          changesets: host.missing_for(graph, args.heads),
          heads: host.heads(graph),
        }));
        await push({ target: '*', changesets, heads });

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
    resolve: ({ changesets, heads }) => ({ changesets, heads }),
  }),
}));
