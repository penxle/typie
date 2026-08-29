import { faker } from '@faker-js/faker';
import { defaultValues } from '@typie/lib/const';
import { EntityState, EntityType, NoteState } from '@typie/lib/enums';
import { TypieError } from '@typie/lib/errors';
import { and, asc, eq, inArray, isNull, ne, sql } from 'drizzle-orm';
import {
  db,
  Dividers,
  DocumentBundles,
  Documents,
  DocumentStates,
  Entities,
  Files,
  first,
  firstOrThrow,
  Folders,
  Images,
  NoteEntities,
  Notes,
} from '#/db/index.ts';
import { readMergedGraph } from '#/utils/changeset.ts';
import { isSnapshotUsable } from '#/utils/document-state.ts';
import { generateFractionalOrder } from '#/utils/order.ts';
import { wasm as wasmFfi } from '#/utils/wasm-ffi.ts';
import type { Modifier, PlainDoc, PlainNodeEntry, PlainRootNode } from '@typie/editor-ffi/server';
import type { Transaction } from '#/db/index.ts';

export const generateSlug = () => faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' });
export const generatePermalink = () => faker.string.alphanumeric({ length: 6, casing: 'mixed' });

export type TemplatePreset = {
  fontFamily?: string;
  fontSize?: number;
  fontWeight?: number;
  textColor?: string;
  backgroundColor?: string;
  letterSpacing?: number;
  lineHeight?: number;
  layout?:
    | { type: 'continuous'; maxWidth: number }
    | {
        type: 'paginated';
        pageWidth: number;
        pageHeight: number;
        pageMarginTop: number;
        pageMarginBottom: number;
        pageMarginLeft: number;
        pageMarginRight: number;
      };
  paragraphIndent?: number;
  blockGap?: number;
};

type ResolvedPreset = {
  fontFamily: string;
  fontSize: number;
  fontWeight: number;
  textColor: string;
  backgroundColor: string;
  letterSpacing: number;
  lineHeight: number;
  paragraphIndent: number;
  blockGap: number;
  layout:
    | { type: 'continuous'; maxWidth: number }
    | {
        type: 'paginated';
        pageWidth: number;
        pageHeight: number;
        pageMarginTop: number;
        pageMarginBottom: number;
        pageMarginLeft: number;
        pageMarginRight: number;
      };
};

export const resolvePreset = (preset?: TemplatePreset): ResolvedPreset => {
  const blockGap =
    preset?.blockGap != null && preset.blockGap < 10 && preset.blockGap !== 0
      ? Math.round(preset.blockGap * 100)
      : (preset?.blockGap ?? defaultValues.blockGap);
  const paragraphIndent =
    preset?.paragraphIndent != null && preset.paragraphIndent < 10 && preset.paragraphIndent !== 0
      ? Math.round(preset.paragraphIndent * 100)
      : (preset?.paragraphIndent ?? defaultValues.paragraphIndent);
  const fontSize =
    preset?.fontSize != null && preset.fontSize < 500 && preset.fontSize > 0
      ? Math.round(preset.fontSize * 100)
      : (preset?.fontSize ?? defaultValues.fontSize);
  const letterSpacing =
    preset?.letterSpacing != null && Math.abs(preset.letterSpacing) < 5 && preset.letterSpacing !== 0
      ? Math.round(preset.letterSpacing * 100)
      : (preset?.letterSpacing ?? defaultValues.letterSpacing);
  const lineHeight =
    preset?.lineHeight != null && preset.lineHeight < 10 && preset.lineHeight !== 0
      ? Math.round(preset.lineHeight * 100)
      : (preset?.lineHeight ?? defaultValues.lineHeight);

  const layout =
    preset?.layout?.type === 'paginated'
      ? {
          type: 'paginated' as const,
          pageWidth: preset.layout.pageWidth,
          pageHeight: preset.layout.pageHeight,
          pageMarginTop: preset.layout.pageMarginTop,
          pageMarginBottom: preset.layout.pageMarginBottom,
          pageMarginLeft: preset.layout.pageMarginLeft,
          pageMarginRight: preset.layout.pageMarginRight,
        }
      : {
          type: 'continuous' as const,
          maxWidth: (preset?.layout?.type === 'continuous' ? preset.layout : defaultValues).maxWidth,
        };

  return {
    fontFamily: preset?.fontFamily ?? defaultValues.fontFamily,
    fontSize,
    fontWeight: preset?.fontWeight ?? defaultValues.fontWeight,
    textColor: preset?.textColor ?? defaultValues.textColor,
    backgroundColor: preset?.backgroundColor ?? defaultValues.backgroundColor,
    letterSpacing,
    lineHeight,
    paragraphIndent,
    blockGap,
    layout,
  };
};

export const derivePlainRootFromPreset = (preset?: TemplatePreset): { root: PlainRootNode; modifiers: Modifier[] } => {
  const r = resolvePreset(preset);

  const root: PlainRootNode = {
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
    { type: 'letter_spacing', value: r.letterSpacing },
    { type: 'line_height', value: r.lineHeight },
    { type: 'paragraph_indent', value: r.paragraphIndent },
    { type: 'block_gap', value: r.blockGap },
  ];

  return { root, modifiers };
};

export const extractAssetIdsFromPlainDoc = (
  plain: PlainDoc,
): { imageIds: string[]; fileIds: string[]; embedIds: string[]; archivedIds: string[] } => {
  const imageIds: string[] = [];
  const fileIds: string[] = [];
  const embedIds: string[] = [];
  const archivedIds: string[] = [];

  const walk = (entry: PlainNodeEntry) => {
    const node = entry.node;
    switch (node.type) {
      case 'image': {
        if (node.id != null) imageIds.push(node.id);
        break;
      }
      case 'file': {
        if (node.id != null) fileIds.push(node.id);
        break;
      }
      case 'embed': {
        if (node.id != null) embedIds.push(node.id);
        break;
      }
      case 'archived': {
        if (node.id != null) archivedIds.push(node.id);
        break;
      }
    }
    for (const child of entry.children) walk(child);
  };
  walk(plain.root);

  return { imageIds, fileIds, embedIds, archivedIds };
};

export const calculateBlobSizeFromAssetIds = async (imageIds: string[], fileIds: string[]): Promise<number> => {
  let totalSize = 0;

  if (imageIds.length > 0) {
    const images = await db.select({ size: Images.size }).from(Images).where(inArray(Images.id, imageIds));
    totalSize += images.reduce((sum, img) => sum + img.size, 0);
  }

  if (fileIds.length > 0) {
    const files = await db.select({ size: Files.size }).from(Files).where(inArray(Files.id, fileIds));
    totalSize += files.reduce((sum, file) => sum + file.size, 0);
  }

  return totalSize;
};

export type DocLayoutMode =
  | {
      type: 'paginated';
      pageWidth: number;
      pageHeight: number;
      pageMarginTop: number;
      pageMarginBottom: number;
      pageMarginLeft: number;
      pageMarginRight: number;
    }
  | { type: 'continuous'; maxWidth: number };

export const extractPlainDocLayoutMode = (plain: PlainDoc): DocLayoutMode => {
  const node = plain.root.node;
  if (node.type !== 'root') {
    return {
      type: 'paginated',
      pageWidth: 794,
      pageHeight: 1123,
      pageMarginTop: 96,
      pageMarginBottom: 96,
      pageMarginLeft: 96,
      pageMarginRight: 96,
    };
  }

  const layoutMode = (node as PlainRootNode).layout_mode;

  if (layoutMode.type === 'continuous') {
    return {
      type: 'continuous',
      maxWidth: layoutMode.max_width,
    };
  }

  return {
    type: 'paginated',
    pageWidth: layoutMode.page_width,
    pageHeight: layoutMode.page_height,
    pageMarginTop: layoutMode.page_margin_top,
    pageMarginBottom: layoutMode.page_margin_bottom,
    pageMarginLeft: layoutMode.page_margin_left,
    pageMarginRight: layoutMode.page_margin_right,
  };
};

export const resolveNameConflict = async (
  tx: Transaction,
  name: string,
  parentEntityId: string | null,
  siteId: string,
  entityType: 'DOCUMENT' | 'FOLDER',
): Promise<string> => {
  const parentCondition = parentEntityId ? eq(Entities.parentId, parentEntityId) : isNull(Entities.parentId);

  let siblingNames: string[];

  if (entityType === 'DOCUMENT') {
    const rows = await tx
      .select({ title: Documents.title })
      .from(Entities)
      .innerJoin(Documents, eq(Entities.id, Documents.entityId))
      .where(and(parentCondition, eq(Entities.siteId, siteId), eq(Entities.state, EntityState.ACTIVE)));
    siblingNames = rows.map((r) => r.title ?? '');
  } else {
    const rows = await tx
      .select({ name: Folders.name })
      .from(Entities)
      .innerJoin(Folders, eq(Entities.id, Folders.entityId))
      .where(and(parentCondition, eq(Entities.siteId, siteId), eq(Entities.state, EntityState.ACTIVE)));
    siblingNames = rows.map((r) => r.name);
  }

  if (!siblingNames.includes(name)) return name;

  let n = 1;
  while (siblingNames.includes(`${name} (${n})`)) {
    n++;
  }
  return `${name} (${n})`;
};

export type FreshV2Content = {
  plain: PlainDoc;
  graph: Uint8Array;
  heads: Uint8Array;
  text: string;
  characterCount: number;
  blobSize: number;
};

export const buildFreshV2Content = async (documentId: string): Promise<FreshV2Content | null> => {
  const sourceState = await db
    .select({ documentId: DocumentStates.documentId, projectionDegraded: DocumentStates.projectionDegraded })
    .from(DocumentStates)
    .where(eq(DocumentStates.documentId, documentId))
    .then(first);

  if (!isSnapshotUsable(sourceState)) {
    return null;
  }

  const mergedGraph = await readMergedGraph(documentId);

  const converted = await wasmFfi.use((host) => {
    const plain = host.to_plain(mergedGraph);
    const graph = host.to_graph(plain);
    const text = host.extract_text(plain);
    return { plain, graph, heads: host.heads(graph), text, characterCount: host.count_characters(text) };
  });

  const { imageIds, fileIds } = extractAssetIdsFromPlainDoc(converted.plain);
  const blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);

  return { ...converted, blobSize };
};

export const insertFreshV2Content = async (tx: Transaction, documentId: string, content: FreshV2Content): Promise<void> => {
  await tx.insert(DocumentBundles).values({ documentId, seq: 1, payload: content.graph });
  await tx.insert(DocumentStates).values({
    documentId,
    json: content.plain,
    text: content.text,
    characterCount: content.characterCount,
    blobSize: content.blobSize,
    heads: content.heads,
    lastBundleSeq: 1,
  });
};

const copyDocument = async (
  tx: Transaction,
  sourceEntityId: string,
  newEntityId: string,
  targetParentId: string | null,
  targetSiteId: string,
  v2Map: Map<string, FreshV2Content>,
) => {
  const sourceDoc = await tx.select().from(Documents).where(eq(Documents.entityId, sourceEntityId)).then(firstOrThrow);

  const resolvedTitle = await resolveNameConflict(tx, sourceDoc.title ?? '(제목 없음)', targetParentId, targetSiteId, 'DOCUMENT');

  const newDoc = await tx
    .insert(Documents)
    .values({
      entityId: newEntityId,
      title: resolvedTitle,
      subtitle: sourceDoc.subtitle,
      password: sourceDoc.password,
      contentRating: sourceDoc.contentRating,
      allowReaction: sourceDoc.allowReaction,
      protectContent: sourceDoc.protectContent,
      locked: sourceDoc.locked,
      thumbnailId: sourceDoc.thumbnailId,
      type: sourceDoc.type,
    })
    .returning()
    .then(firstOrThrow);

  const v2 = v2Map.get(sourceDoc.id) ?? (await buildFreshV2Content(sourceDoc.id));
  if (!v2) {
    throw new TypieError({ code: 'document_projection_degraded', message: '문서를 복사할 수 없는 상태예요.', status: 409 });
  }

  await insertFreshV2Content(tx, newDoc.id, v2);

  return newDoc;
};

const copyFolder = async (
  tx: Transaction,
  sourceEntityId: string,
  newEntityId: string,
  newName: string,
  targetSiteId: string,
  parentDepth: number,
  userId: string,
  v2Map: Map<string, FreshV2Content>,
) => {
  const sourceFolder = await tx.select().from(Folders).where(eq(Folders.entityId, sourceEntityId)).then(firstOrThrow);

  await tx.insert(Folders).values({
    entityId: newEntityId,
    name: newName,
    thumbnailId: sourceFolder.thumbnailId,
  });

  const children = await tx
    .select()
    .from(Entities)
    .where(and(eq(Entities.parentId, sourceEntityId), eq(Entities.state, EntityState.ACTIVE), ne(Entities.id, newEntityId)))
    .orderBy(asc(Entities.order));

  let prevOrder: string | undefined;
  for (const child of children) {
    const childOrder = generateFractionalOrder({ lower: prevOrder, upper: undefined });
    await copyEntityRecursive(tx, child.id, targetSiteId, newEntityId, parentDepth + 1, childOrder, userId, v2Map);
    prevOrder = childOrder;
  }
};

export const copyEntityRecursive = async (
  tx: Transaction,
  sourceEntityId: string,
  targetSiteId: string,
  targetParentId: string | null,
  targetDepth: number,
  order: string,
  userId: string,
  v2Map: Map<string, FreshV2Content>,
): Promise<string> => {
  const sourceEntity = await tx.select().from(Entities).where(eq(Entities.id, sourceEntityId)).then(firstOrThrow);

  const newEntity = await tx
    .insert(Entities)
    .values({
      userId,
      siteId: targetSiteId,
      parentId: targetParentId,
      slug: generateSlug(),
      permalink: generatePermalink(),
      type: sourceEntity.type,
      order,
      depth: targetDepth,
      state: sourceEntity.state,
      visibility: sourceEntity.visibility,
      availability: sourceEntity.availability,
      icon: sourceEntity.icon,
      iconColor: sourceEntity.iconColor,
    })
    .returning()
    .then(firstOrThrow);

  if (sourceEntity.type === EntityType.DOCUMENT) {
    await copyDocument(tx, sourceEntityId, newEntity.id, targetParentId, targetSiteId, v2Map);
  } else if (sourceEntity.type === EntityType.FOLDER) {
    const sourceFolder = await tx.select().from(Folders).where(eq(Folders.entityId, sourceEntityId)).then(firstOrThrow);
    const resolvedName = await resolveNameConflict(tx, sourceFolder.name, targetParentId, targetSiteId, 'FOLDER');
    await copyFolder(tx, sourceEntityId, newEntity.id, resolvedName, targetSiteId, targetDepth, userId, v2Map);
  } else if (sourceEntity.type === EntityType.DIVIDER) {
    await tx.insert(Dividers).values({ entityId: newEntity.id });
  }

  // Notes 복제
  const noteRows = await tx
    .select({
      content: Notes.content,
      color: Notes.color,
      status: Notes.status,
    })
    .from(NoteEntities)
    .innerJoin(Notes, eq(NoteEntities.noteId, Notes.id))
    .where(and(eq(NoteEntities.entityId, sourceEntityId), eq(Notes.state, NoteState.ACTIVE)));

  if (noteRows.length > 0) {
    let prevNoteOrder: string | null = null;

    for (const row of noteRows) {
      const noteOrder = generateFractionalOrder({ lower: prevNoteOrder, upper: null });

      const newNote = await tx
        .insert(Notes)
        .values({
          userId,
          siteId: targetSiteId,
          content: row.content,
          color: row.color,
          status: row.status,
          order: noteOrder,
        })
        .returning({ id: Notes.id })
        .then(firstOrThrow);

      await tx.insert(NoteEntities).values({
        noteId: newNote.id,
        entityId: newEntity.id,
      });

      prevNoteOrder = noteOrder;
    }
  }

  return newEntity.id;
};

export const getAncestorEntityIds = async (entityId: string): Promise<string[]> => {
  const result = await db.execute<{ id: string }>(sql`
    WITH RECURSIVE ancestors AS (
      SELECT id, parent_id FROM entities WHERE id = ${entityId}
      UNION ALL
      SELECT e.id, e.parent_id FROM entities e
      INNER JOIN ancestors a ON a.parent_id = e.id
    )
    SELECT id FROM ancestors WHERE id != ${entityId}
  `);

  return result.map((row) => row.id);
};
