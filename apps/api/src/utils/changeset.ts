import { eq } from 'drizzle-orm';
import { redis } from '#/cache.ts';
import { db, DocumentStates, firstOrThrow } from '#/db/index.ts';
import { wasm } from './wasm-ffi.ts';

export const readMergedGraph = async (documentId: string): Promise<Uint8Array> => {
  const pending = await redis.lrange(`document:changesets:pending:${documentId}`, 0, -1);

  const { graph: persisted } = await db
    .select({ graph: DocumentStates.graph })
    .from(DocumentStates)
    .where(eq(DocumentStates.documentId, documentId))
    .then(firstOrThrow);

  if (pending.length === 0) return persisted;

  const pendingBundles = pending.toReversed().map((p) => Uint8Array.fromBase64((JSON.parse(p) as { payload: string }).payload));

  return await wasm.use((host) => {
    let merged = persisted;
    for (const bundle of pendingBundles) {
      merged = host.apply(merged, bundle);
    }
    return merged;
  });
};
