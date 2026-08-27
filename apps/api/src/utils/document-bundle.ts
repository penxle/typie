import { enqueueJob } from '#/mq/index.ts';
import { pubsub } from '#/pubsub.ts';
import { appendBundle, getDurableHeads, readMergedGraph, setLiveHeads } from '#/utils/changeset.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';

export const publishBundle = async (
  documentId: string,
  bundle: Uint8Array,
  userId: string,
  deviceId: string,
): Promise<{ seq: string; heads: Uint8Array }> => {
  const seq = await appendBundle(documentId, bundle, userId, deviceId);

  const mergedGraph = await readMergedGraph(documentId);
  const heads = await wasmFfi.use((host) => host.heads(mergedGraph));
  // No wasm recompute: the durable frontier is whatever collect has folded
  // into `document_states.heads` so far — the bundle itself only
  // affects it once collect processes this push, same as any other push.
  const durableHeads = (await getDurableHeads(documentId)) ?? new Uint8Array();

  await setLiveHeads(documentId, heads);

  pubsub.publish('document:changesets', documentId, {
    target: '*',
    seq,
    changesets: [bundle.toBase64()],
    heads: heads.toBase64(),
    durableHeads: durableHeads.toBase64(),
  });

  await enqueueJob('document:changesets:collect', documentId);

  return { seq, heads };
};
