import { eq, inArray, sql } from 'drizzle-orm';
import { DocumentCommits, DocumentObjects, firstOrThrow } from '#/db/index.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import type { Doc, ObjectContent } from '@typie/editor-ffi/server';
import type { Database, Transaction } from '#/db/index.ts';

export async function walkReachableObjects(
  tx: Database | Transaction,
  rootObjectId: string,
): Promise<(typeof DocumentObjects.$inferSelect)[]> {
  const idRows = await tx.execute<{ id: string }>(sql`
    WITH RECURSIVE reachable AS (
      SELECT id, content FROM ${DocumentObjects} WHERE id = ${rootObjectId}
      UNION
      SELECT o.id, o.content
      FROM ${DocumentObjects} o
      JOIN reachable r ON o.hash IN (
        SELECT cp->>'hash' FROM jsonb_array_elements(r.content->'children') cp
      )
    )
    SELECT id FROM reachable
  `);
  if (idRows.length === 0) return [];
  return tx
    .select()
    .from(DocumentObjects)
    .where(
      inArray(
        DocumentObjects.id,
        idRows.map((r) => r.id),
      ),
    );
}

export async function walkReachableHashes(tx: Database | Transaction, rootObjectId: string): Promise<Set<string>> {
  const rows = await tx.execute<{ hash: string }>(sql`
    WITH RECURSIVE reachable AS (
      SELECT id, hash, content FROM ${DocumentObjects} WHERE id = ${rootObjectId}
      UNION
      SELECT o.id, o.hash, o.content
      FROM ${DocumentObjects} o
      JOIN reachable r ON o.hash IN (
        SELECT cp->>'hash' FROM jsonb_array_elements(r.content->'children') cp
      )
    )
    SELECT hash FROM reachable
  `);
  return new Set(rows.map((r) => r.hash));
}

export async function loadDocFromObjectId(tx: Database | Transaction, objectId: string): Promise<{ rootHash: string; doc: Doc }> {
  const root = await tx.select().from(DocumentObjects).where(eq(DocumentObjects.id, objectId)).then(firstOrThrow);
  const allHashes = await walkReachableHashes(tx, objectId);
  const allObjects = await tx
    .select({ hash: DocumentObjects.hash, content: DocumentObjects.content })
    .from(DocumentObjects)
    .where(inArray(DocumentObjects.hash, [...allHashes]));
  const doc = await wasm.reconstruct_doc_from_objects(
    root.hash,
    allObjects.map((o) => ({ hash: o.hash, content: o.content as ObjectContent })),
  );
  return { rootHash: root.hash, doc };
}

export async function isAncestor(tx: Database | Transaction, ancestorCommitId: string, descendantCommitId: string): Promise<boolean> {
  if (ancestorCommitId === descendantCommitId) return true;
  const rows = await tx.execute<{ id: string }>(sql`
    WITH RECURSIVE ancestors AS (
      SELECT id, parent_id, second_parent_id FROM ${DocumentCommits} WHERE id = ${descendantCommitId}
      UNION
      SELECT c.id, c.parent_id, c.second_parent_id
      FROM ${DocumentCommits} c
      JOIN ancestors a ON c.id = a.parent_id OR c.id = a.second_parent_id
    )
    SELECT id FROM ancestors WHERE id = ${ancestorCommitId} LIMIT 1
  `);
  return rows.length > 0;
}
