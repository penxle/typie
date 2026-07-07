import { TypieError } from '@typie/lib/errors';
import { eq } from 'drizzle-orm';
import { filter, pipe, Repeater } from 'graphql-yoga';
import { db, Documents, DocumentStates, Entities, first, firstOrThrow, TableCode, validateDbId } from '#/db/index.ts';
import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import {
  advanceLiveHeads,
  appendBundle,
  getCollectedSeq,
  getDurableHeads,
  getLiveHeads,
  loadBundleStream,
  readMergedGraph,
  readStreamSince,
  setLiveHeads,
} from '#/utils/changeset.ts';
import { assertSitePermission } from '#/utils/permission.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import { builder } from '../builder.ts';
import { Document, DocumentState } from '../objects.ts';

/**
 * * Types
 */

DocumentState.implement({
  fields: (t) => ({
    // The persisted snapshot as-is — NO folding of the un-collected stream tail.
    // Folding it here meant an `O(tail × N)` `host.apply` chain (each call
    // re-decodes the whole multi-MB graph) that ballooned to tens of seconds
    // whenever collect fell behind. The client loads this snapshot and catches
    // the tail up with a single `O(tail)` seq-pull instead.
    graph: t.field({
      type: 'Binary',
      resolve: (self) => loadBundleStream(self.documentId),
    }),
    // Cursor the snapshot corresponds to: the client resumes incremental sync
    // here and pulls only what the snapshot is missing. Empty ⇒ start from the
    // stream origin.
    seq: t.field({
      type: 'String',
      resolve: (self) => getCollectedSeq(self.documentId).then((seq) => seq ?? ''),
    }),
    // Frontier of the snapshot (matches `graph`): `self` is already the loaded
    // `document_states` row (see `Document.state` below), so its `heads` column
    // — the durable source of truth — needs no further lookup.
    heads: t.field({
      type: 'Binary',
      resolve: (self) => self.heads,
    }),
    durableHeads: t.field({
      type: 'Binary',
      resolve: (self) => self.heads,
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

      let seq: string | null = null;
      if (opsCount > 0) {
        seq = await appendBundle(input.documentId, input.changesets, ctx.session.userId, ctx.session.deviceId);
      }

      // The pusher needs the accurate live frontier (to avoid re-broadcasting
      // peers' changesets); the read paths (pull / catch-up / load) consume the
      // cached value. Warm path: fold just this bundle into the cached frontier
      // — `O(bundle)`. The old rebuild (fetch the multi-MB snapshot, merge the
      // stream tail, re-scan the whole graph for heads) made every push on a
      // large document `O(history)` — ~3 s of blocking wasm per push at 8MB.
      let heads = opsCount > 0 ? await advanceLiveHeads(input.documentId, input.changesets) : await getLiveHeads(input.documentId);
      // `getDurableHeads` reads `document_states.heads` directly — no cache to
      // bootstrap. Empty bytes is only reachable for a document with no
      // persisted state at all.
      const durableHeads = (await getDurableHeads(input.documentId)) ?? new Uint8Array();

      // Cold cache (fresh document, Redis flush, or pre-cache deploys):
      // bootstrap the live frontier once via the merged graph, then the warm
      // path above keeps it current.
      if (!heads) {
        const graph = await readMergedGraph(input.documentId);
        heads = await wasm.use((host) => host.heads(graph));
        await setLiveHeads(input.documentId, heads);
      }

      if (opsCount > 0 && seq) {
        pubsub.publish('document:changesets', input.documentId, {
          target: `!${input.clientId}`,
          seq,
          changesets: [input.changesets.toBase64()],
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
        changesets: t.field({ type: ['Binary'] }),
        seq: t.field({ type: 'String' }),
        heads: t.field({ type: 'Binary' }),
        durableHeads: t.field({ type: 'Binary' }),
        needsReload: t.field({ type: 'Boolean' }),
      }),
    }),
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      sinceSeq: t.input.string({ required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const docEntity = await db
        .select({ siteId: Entities.siteId })
        .from(Documents)
        .innerJoin(Entities, eq(Documents.entityId, Entities.id))
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrow);

      await assertSitePermission({ userId: ctx.session.userId, siteId: docEntity.siteId });

      // O(missing) tail read from the shared stream — no graph rebuild.
      const { entries, tip, truncated } = await readStreamSince(input.documentId, input.sinceSeq ?? null);

      const [live, durable] = await Promise.all([getLiveHeads(input.documentId), getDurableHeads(input.documentId)]);
      const durableHeads = durable ?? new Uint8Array();
      const heads = live ?? durableHeads;

      if (truncated) {
        // The client's cursor fell out of the retained window: it must reload
        // the full document (and restart sync from the fresh seq) rather than
        // incrementally catch up on entries that were already trimmed.
        return { changesets: [], seq: input.sinceSeq ?? '', heads, durableHeads, needsReload: true };
      }

      return {
        changesets: entries.map((e) => e.changeset),
        seq: tip ?? input.sinceSeq ?? '',
        heads,
        durableHeads,
        needsReload: false,
      };
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
        changesets: t.field({ type: ['Binary'] }),
        seq: t.field({ type: 'String' }),
        heads: t.field({ type: 'Binary' }),
        durableHeads: t.field({ type: 'Binary' }),
      }),
    }),
    args: {
      documentId: t.arg.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      clientId: t.arg.string(),
      sinceSeq: t.arg.string({ required: false }),
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

      type Event = { target: string; seq: string; changesets: Uint8Array[]; heads: Uint8Array; durableHeads: Uint8Array };

      const repeater = new Repeater<Event>(async (push, stop) => {
        const liveBuffer: Event[] = [];
        let catchupComplete = false;

        const liveStream = pubsub.subscribe('document:changesets', args.documentId);

        const livePromise = (async () => {
          for await (const event of liveStream) {
            const decoded: Event = {
              target: event.target,
              seq: event.seq,
              changesets: event.changesets.map((c) => Uint8Array.fromBase64(c)),
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

        // Catch-up is an O(missing) tail read from the client's cursor — no
        // graph rebuild. Beyond the retained window the client full-reloads
        // (its poll's `needsReload`), so an empty catch-up here is safe.
        const { entries, tip, truncated } = await readStreamSince(args.documentId, args.sinceSeq ?? null);
        const [live, durable] = await Promise.all([getLiveHeads(args.documentId), getDurableHeads(args.documentId)]);
        const durableHeads = durable ?? new Uint8Array();
        await push({
          target: '*',
          seq: truncated ? '' : (tip ?? args.sinceSeq ?? ''),
          changesets: truncated ? [] : entries.map((e) => e.changeset),
          heads: live ?? durableHeads,
          durableHeads,
        });

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
    resolve: ({ changesets, seq, heads, durableHeads }) => ({ changesets, seq, heads, durableHeads }),
  }),
}));
