import { compareStreamSeq } from './protocol.ts';
import type { ServerMessage } from './protocol.ts';
import type { SyncDeps, SyncSession } from './types.ts';

type SendFn = (message: ServerMessage) => Promise<void>;

export type RequestContext = { deps: SyncDeps; session: SyncSession; clientId: string };

export const PULL_MAX_ENTRIES = 64;
export const PULL_MAX_BYTES = 4 * 1024 * 1024;

export const handlePush = async (
  ctx: RequestContext,
  message: { id: string; documentId: string; changesets: Uint8Array },
  send: SendFn,
): Promise<void> => {
  let opsCount: number;
  try {
    opsCount = await ctx.deps.peekOpsCount(message.changesets);
  } catch {
    await send({ t: 'error', scope: 'request', id: message.id, code: 'invalid_changeset_payload', permanent: true });
    return;
  }

  let seq: string | null = null;
  if (opsCount > 0) {
    seq = await ctx.deps.appendBundle(message.documentId, message.changesets, ctx.session.userId, ctx.session.deviceId);
  }

  let heads =
    opsCount > 0
      ? await ctx.deps.advanceLiveHeads(message.documentId, message.changesets)
      : await ctx.deps.getLiveHeads(message.documentId);
  const durableHeads = (await ctx.deps.getDurableHeads(message.documentId)) ?? new Uint8Array();
  heads ??= await ctx.deps.bootstrapLiveHeads(message.documentId);

  if (opsCount > 0 && seq) {
    const headsB64 = heads.toBase64();
    const durableHeadsB64 = durableHeads.toBase64();
    ctx.deps.publishChangesets(message.documentId, {
      target: `!${ctx.clientId}`,
      seq,
      changesets: [message.changesets.toBase64()],
      heads: headsB64,
      durableHeads: durableHeadsB64,
    });
    ctx.deps.publishChangesets(message.documentId, {
      target: ctx.clientId,
      seq,
      changesets: [],
      heads: headsB64,
      durableHeads: durableHeadsB64,
    });
    await ctx.deps.enqueueCollect(message.documentId);
    await ctx.deps.markWriterActive(ctx.session.userId);
  }

  await send({ t: 'push-ack', id: message.id, heads, durableHeads });
};

export const handlePull = async (
  ctx: RequestContext,
  message: { id: string; documentId: string; sinceSeq?: string },
  send: SendFn,
): Promise<void> => {
  const sinceSeq = message.sinceSeq || null;
  const [live, durable] = await Promise.all([ctx.deps.getLiveHeads(message.documentId), ctx.deps.getDurableHeads(message.documentId)]);
  const durableHeads = durable ?? new Uint8Array();
  const heads = live ?? durableHeads;
  const reload = () => send({ t: 'pull-ack', id: message.id, changesets: [], seq: sinceSeq ?? '', heads, durableHeads, needsReload: true });

  if (sinceSeq === null) {
    if ((await ctx.deps.getCollectedSeq(message.documentId)) !== null) {
      const tip = await ctx.deps.streamTip(message.documentId);
      if (tip === null || (await ctx.deps.hasStreamBeenTrimmed(message.documentId))) {
        await reload();
        return;
      }
    }
  } else {
    const tip = await ctx.deps.streamTip(message.documentId);
    if (tip === null) {
      // Same caught-up carve-out as the channel attach path: an expired idle
      // stream only ever drops collected entries, so a cursor exactly at
      // collectedSeq needs no reload — the empty page below acks it as-is.
      const collectedSeq = await ctx.deps.getCollectedSeq(message.documentId);
      if (collectedSeq === null || compareStreamSeq(sinceSeq, collectedSeq) !== 0) {
        await reload();
        return;
      }
    } else if (compareStreamSeq(sinceSeq, tip) > 0) {
      await reload();
      return;
    }
  }

  const page = await ctx.deps.readStreamBatch(message.documentId, sinceSeq, PULL_MAX_ENTRIES + 1);
  if (sinceSeq !== null && (await ctx.deps.isStreamTruncated(message.documentId, sinceSeq))) {
    await reload();
    return;
  }
  const bytes = page.reduce((n, e) => n + e.changeset.length, 0);
  if (page.length > PULL_MAX_ENTRIES || bytes > PULL_MAX_BYTES) {
    await reload();
    return;
  }

  await send({
    t: 'pull-ack',
    id: message.id,
    changesets: page.map((e) => e.changeset),
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
    seq: page.length > 0 ? page.at(-1)!.seq : (sinceSeq ?? ''),
    heads,
    durableHeads,
    needsReload: false,
  });
};
