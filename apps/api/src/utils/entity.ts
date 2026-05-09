import { faker } from '@faker-js/faker';
import { defaultValues } from '@typie/lib/const';
import { EntityState, EntityType, NoteState } from '@typie/lib/enums';
import { and, asc, eq, inArray, isNull, ne, sql } from 'drizzle-orm';
import { LoroDoc, LoroList, LoroMap } from 'loro-crdt';
import {
  db,
  DocumentContents,
  Documents,
  DocumentVersionContributors,
  DocumentVersions,
  Entities,
  Files,
  firstOrThrow,
  Folders,
  Images,
  NoteEntities,
  Notes,
} from '#/db/index.ts';
import { compressZstd } from '#/utils/compression.ts';
import { generateFractionalOrder } from '#/utils/order.ts';
import { wasm } from '#/utils/wasm.ts';
import type { Modifier, PlainDoc, PlainRootNode } from '@typie/editor-ffi/server';
import type { Transaction } from '#/db/index.ts';

export const generateSlug = () => faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' });
export const generatePermalink = () => faker.string.alphanumeric({ length: 6, casing: 'mixed' });

const ROOT_ID = '00000000000000000000000000000000';

const collectReachableNodeIds = (nodes: Record<string, { children?: string[] }>): Set<string> => {
  const reachable = new Set<string>();
  const traverse = (nodeId: string) => {
    if (reachable.has(nodeId)) return;
    reachable.add(nodeId);
    const node = nodes[nodeId];
    if (!node?.children) return;
    for (const childId of node.children) {
      traverse(childId);
    }
  };
  traverse(ROOT_ID);
  return reachable;
};

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
          maxWidth: preset?.layout?.type === 'continuous' ? preset.layout.maxWidth : defaultValues.maxWidth,
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

export const makeLoroDoc = (template?: TemplatePreset) => {
  const doc = new LoroDoc();

  const r = resolvePreset(template);

  const settings = doc.getMap('settings');
  settings.set('block_gap', r.blockGap);
  settings.set('paragraph_indent', r.paragraphIndent);

  const loroLayout = settings.setContainer('layout_mode', new LoroMap());
  if (r.layout.type === 'paginated') {
    loroLayout.set('type', 'paginated');
    loroLayout.set('page_width', r.layout.pageWidth);
    loroLayout.set('page_height', r.layout.pageHeight);
    loroLayout.set('page_margin_top', r.layout.pageMarginTop);
    loroLayout.set('page_margin_bottom', r.layout.pageMarginBottom);
    loroLayout.set('page_margin_left', r.layout.pageMarginLeft);
    loroLayout.set('page_margin_right', r.layout.pageMarginRight);
  } else {
    loroLayout.set('type', 'continuous');
    loroLayout.set('max_width', r.layout.maxWidth);
  }

  const paragraphId = faker.string.uuid().replaceAll('-', '');

  const nodes = doc.getMap('nodes');

  const rootNode = nodes.setContainer(ROOT_ID, new LoroMap());
  rootNode.set('type', 'root');
  const rootChildren = rootNode.setContainer('children', new LoroList());
  rootChildren.insert(0, paragraphId);

  const cascadeAttrs = rootNode.setContainer('cascade_attrs', new LoroMap());
  cascadeAttrs.set('style:font_family', r.fontFamily);
  cascadeAttrs.set('style:font_size', r.fontSize);
  cascadeAttrs.set('style:font_weight', r.fontWeight);
  cascadeAttrs.set('style:text_color', r.textColor);
  cascadeAttrs.set('style:background_color', r.backgroundColor);
  cascadeAttrs.set('style:letter_spacing', r.letterSpacing);
  cascadeAttrs.set('paragraph:line_height', r.lineHeight);

  const paragraphNode = nodes.setContainer(paragraphId, new LoroMap());
  paragraphNode.set('type', 'paragraph');
  paragraphNode.set('align', defaultValues.textAlign);
  paragraphNode.set('line_height', r.lineHeight);
  paragraphNode.set('parent', ROOT_ID);
  paragraphNode.setContainer('children', new LoroList());

  return doc;
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
    { type: 'text_color', value: r.textColor },
    { type: 'background_color', value: r.backgroundColor },
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

  for (const entry of Object.values(plain.nodes)) {
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
  }

  return { imageIds, fileIds, embedIds, archivedIds };
};

const extractTextFromLoroDoc = (doc: LoroDoc): string => {
  const nodes = doc.getMap('nodes').toJSON() as Record<string, { text?: string; children?: string[] }>;
  const reachable = collectReachableNodeIds(nodes);
  const texts: string[] = [];

  for (const nodeId of reachable) {
    const node = nodes[nodeId];
    if (node?.text) {
      texts.push(node.text);
    }
  }

  return texts.join('');
};

export const extractAssetIdsFromLoroDoc = (
  doc: LoroDoc,
): { imageIds: string[]; fileIds: string[]; embedIds: string[]; archivedIds: string[] } => {
  const allNodes = doc.getMap('nodes').toJSON() as Record<string, unknown>;
  const reachable = collectReachableNodeIds(allNodes as Record<string, { children?: string[] }>);

  const imageIds: string[] = [];
  const fileIds: string[] = [];
  const embedIds: string[] = [];
  const archivedIds: string[] = [];

  for (const nodeId of reachable) {
    const typedNode = allNodes[nodeId] as { type?: string; id?: string };
    if (typedNode.type === 'image' && typedNode.id) {
      imageIds.push(typedNode.id);
    } else if (typedNode.type === 'file' && typedNode.id) {
      fileIds.push(typedNode.id);
    } else if (typedNode.type === 'embed' && typedNode.id) {
      embedIds.push(typedNode.id);
    } else if (typedNode.type === 'archived' && typedNode.id) {
      archivedIds.push(typedNode.id);
    }
  }

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

export const garbageCollectLoroDoc = (doc: LoroDoc): number => {
  const nodes = doc.getMap('nodes');
  const allNodes = nodes.toJSON() as Record<string, { children?: string[]; text?: unknown }>;
  const reachable = collectReachableNodeIds(allNodes as Record<string, { children?: string[] }>);

  let deletedNodes = 0;
  for (const key of nodes.keys()) {
    if (!reachable.has(key)) {
      nodes.delete(key);
      deletedNodes++;
    }
  }

  return deletedNodes;
};

export const countCharacters = (text: string) => {
  return [...text.replaceAll('\u200B', '').replaceAll(/\s+/g, ' ').trim()].length;
};

export const extractLoroDocContents = async (doc: LoroDoc) => {
  const snapshot = new Uint8Array(doc.export({ mode: 'snapshot' }));
  const json = await wasm.snapshotToJson(snapshot);
  const text = extractTextFromLoroDoc(doc);
  const characterCount = countCharacters(text);
  const { imageIds, fileIds } = extractAssetIdsFromLoroDoc(doc);
  const blobSize = await calculateBlobSizeFromAssetIds(imageIds, fileIds);

  return { json, text, characterCount, blobSize };
};

export type LoroLayoutMode =
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

export const extractLoroDocLayoutMode = (snapshot: Uint8Array): LoroLayoutMode => {
  const doc = new LoroDoc();
  doc.import(snapshot);

  const settings = doc.getMap('settings');
  const layoutMode = settings.get('layout_mode') as LoroMap | undefined;

  if (!layoutMode) {
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

  const type = layoutMode.get('type') as string;

  if (type === 'continuous') {
    return {
      type: 'continuous',
      maxWidth: (layoutMode.get('max_width') as number) ?? 600,
    };
  }

  return {
    type: 'paginated',
    pageWidth: (layoutMode.get('page_width') as number) ?? 794,
    pageHeight: (layoutMode.get('page_height') as number) ?? 1123,
    pageMarginTop: (layoutMode.get('page_margin_top') as number) ?? 96,
    pageMarginBottom: (layoutMode.get('page_margin_bottom') as number) ?? 96,
    pageMarginLeft: (layoutMode.get('page_margin_left') as number) ?? 96,
    pageMarginRight: (layoutMode.get('page_margin_right') as number) ?? 96,
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

const copyDocument = async (
  tx: Transaction,
  sourceEntityId: string,
  newEntityId: string,
  targetParentId: string | null,
  targetSiteId: string,
  userId: string,
) => {
  const sourceDoc = await tx.select().from(Documents).where(eq(Documents.entityId, sourceEntityId)).then(firstOrThrow);

  const sourceContent = await tx.select().from(DocumentContents).where(eq(DocumentContents.documentId, sourceDoc.id)).then(firstOrThrow);

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

  const json = await wasm.snapshotToJson(new Uint8Array(sourceContent.snapshot));
  const freshSnapshot = await wasm.jsonToSnapshot(json);
  const freshDoc = new LoroDoc();
  freshDoc.import(freshSnapshot);
  const freshVersion = freshDoc.version().encode();

  await tx.insert(DocumentContents).values({
    documentId: newDoc.id,
    json,
    text: sourceContent.text,
    characterCount: sourceContent.characterCount,
    blobSize: sourceContent.blobSize,
    snapshot: freshSnapshot,
    version: freshVersion,
  });

  const documentVersion = await tx
    .insert(DocumentVersions)
    .values({
      documentId: newDoc.id,
      version: await compressZstd(freshVersion),
    })
    .returning({ id: DocumentVersions.id })
    .then(firstOrThrow);

  await tx.insert(DocumentVersionContributors).values({
    versionId: documentVersion.id,
    userId,
  });

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
    await copyEntityRecursive(tx, child.id, targetSiteId, newEntityId, parentDepth + 1, childOrder, userId);
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
    await copyDocument(tx, sourceEntityId, newEntity.id, targetParentId, targetSiteId, userId);
  } else if (sourceEntity.type === EntityType.FOLDER) {
    const sourceFolder = await tx.select().from(Folders).where(eq(Folders.entityId, sourceEntityId)).then(firstOrThrow);
    const resolvedName = await resolveNameConflict(tx, sourceFolder.name, targetParentId, targetSiteId, 'FOLDER');
    await copyFolder(tx, sourceEntityId, newEntity.id, resolvedName, targetSiteId, targetDepth, userId);
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
