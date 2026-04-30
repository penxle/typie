import { eq, inArray, sql } from 'drizzle-orm';
import { DocumentCommits, DocumentObjects, firstOrThrow } from '#/db/index.ts';
import { calculateBlobSizeFromAssetIds, countCharacters, resolvePreset } from '#/utils/entity.ts';
import { wasm } from '#/utils/wasm-ffi.ts';
import type { CommitObject, Doc, Modifier, ObjectContent, RootNode } from '@typie/editor-ffi/server';
import type { Database, Transaction } from '#/db/index.ts';
import type { TemplatePreset } from '#/utils/entity.ts';

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

export async function loadDocFromRootObjectId(tx: Database | Transaction, rootObjectId: string): Promise<{ rootHash: string; doc: Doc }> {
  const root = await tx.select().from(DocumentObjects).where(eq(DocumentObjects.id, rootObjectId)).then(firstOrThrow);
  const allHashes = await walkReachableHashes(tx, rootObjectId);
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

export type InitialDocBundle = {
  doc: Doc;
  rootHash: string;
  objects: CommitObject[];
  text: string;
  characterCount: number;
  blobSize: number;
};

export const buildInitialDocFromPreset = async (preset?: TemplatePreset): Promise<InitialDocBundle> => {
  const r = resolvePreset(preset);

  const root: RootNode = {
    layout_mode:
      r.layout.type === 'paginated'
        ? {
            type: 'paginated',
            page_width: r.layout.pageWidth,
            page_height: r.layout.pageHeight,
            page_margin_top: r.layout.pageMarginTop,
            page_margin_bottom: r.layout.pageMarginBottom,
            page_margin_left: r.layout.pageMarginLeft,
            page_margin_right: r.layout.pageMarginRight,
          }
        : { type: 'continuous', max_width: r.layout.maxWidth },
  };

  const modifiers: Modifier[] = [
    { type: 'font_family', value: r.fontFamily },
    { type: 'font_size', value: r.fontSize },
    { type: 'font_weight', value: r.fontWeight },
    { type: 'text_color', value: r.textColor },
    { type: 'background_color', value: r.backgroundColor },
    { type: 'letter_spacing', value: r.letterSpacing },
    { type: 'line_height', value: r.lineHeight },
    { type: 'paragraph_indent', value: r.paragraphIndent },
    { type: 'block_gap', value: r.blockGap },
  ];

  const doc = await wasm.default_doc_with_preset(root, modifiers);
  const { rootHash, objects } = await wasm.derive_all_objects(doc);
  const text = await wasm.extract_text(doc);
  const characterCount = countCharacters(text);

  const imageIds: string[] = [];
  const fileIds: string[] = [];
  for (const obj of objects) {
    const node = obj.content.node;
    if (node.type === 'image' && node.id) imageIds.push(node.id);
    else if (node.type === 'file' && node.id) fileIds.push(node.id);
  }
  const blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);

  return { doc, rootHash, objects, text, characterCount, blobSize };
};

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
