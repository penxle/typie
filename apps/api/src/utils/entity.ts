import { faker } from '@faker-js/faker';
import { inArray } from 'drizzle-orm';
import { LoroDoc, LoroList, LoroMap } from 'loro-crdt';
import { defaultValues } from '@/const';
import { db, Files, Images } from '@/db';
import { wasm } from '@/utils/wasm';

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

type TemplatePreset = {
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

export const makeLoroDoc = (template?: TemplatePreset) => {
  const doc = new LoroDoc();

  const blockGap =
    template?.blockGap != null && template.blockGap < 10 && template.blockGap !== 0
      ? Math.round(template.blockGap * 100)
      : (template?.blockGap ?? defaultValues.blockGap);
  const paragraphIndent =
    template?.paragraphIndent != null && template.paragraphIndent < 10 && template.paragraphIndent !== 0
      ? Math.round(template.paragraphIndent * 100)
      : (template?.paragraphIndent ?? defaultValues.paragraphIndent);
  const fontSize =
    template?.fontSize != null && template.fontSize < 500 && template.fontSize > 0
      ? Math.round(template.fontSize * 100)
      : (template?.fontSize ?? defaultValues.fontSize);
  const letterSpacing =
    template?.letterSpacing != null && Math.abs(template.letterSpacing) < 5 && template.letterSpacing !== 0
      ? Math.round(template.letterSpacing * 100)
      : (template?.letterSpacing ?? defaultValues.letterSpacing);
  const lineHeight =
    template?.lineHeight != null && template.lineHeight < 10 && template.lineHeight !== 0
      ? Math.round(template.lineHeight * 100)
      : (template?.lineHeight ?? defaultValues.lineHeight);

  const settings = doc.getMap('settings');
  settings.set('block_gap', blockGap);
  settings.set('paragraph_indent', paragraphIndent);

  const loroLayout = settings.setContainer('layout_mode', new LoroMap());
  if (template?.layout?.type === 'paginated') {
    loroLayout.set('type', 'paginated');
    loroLayout.set('page_width', template.layout.pageWidth);
    loroLayout.set('page_height', template.layout.pageHeight);
    loroLayout.set('page_margin_top', template.layout.pageMarginTop);
    loroLayout.set('page_margin_bottom', template.layout.pageMarginBottom);
    loroLayout.set('page_margin_left', template.layout.pageMarginLeft);
    loroLayout.set('page_margin_right', template.layout.pageMarginRight);
  } else {
    loroLayout.set('type', 'continuous');
    loroLayout.set('max_width', template?.layout?.type === 'continuous' ? template.layout.maxWidth : defaultValues.maxWidth);
  }

  const paragraphId = faker.string.uuid().replaceAll('-', '');

  const nodes = doc.getMap('nodes');

  const rootNode = nodes.setContainer(ROOT_ID, new LoroMap());
  rootNode.set('type', 'root');
  const rootChildren = rootNode.setContainer('children', new LoroList());
  rootChildren.insert(0, paragraphId);

  const cascadeAttrs = rootNode.setContainer('cascade_attrs', new LoroMap());
  cascadeAttrs.set('style:font_family', template?.fontFamily ?? defaultValues.fontFamily);
  cascadeAttrs.set('style:font_size', fontSize);
  cascadeAttrs.set('style:font_weight', template?.fontWeight ?? defaultValues.fontWeight);
  cascadeAttrs.set('style:text_color', template?.textColor ?? defaultValues.textColor);
  cascadeAttrs.set('style:background_color', template?.backgroundColor ?? defaultValues.backgroundColor);
  cascadeAttrs.set('style:letter_spacing', letterSpacing);
  cascadeAttrs.set('paragraph:line_height', lineHeight);

  const paragraphNode = nodes.setContainer(paragraphId, new LoroMap());
  paragraphNode.set('type', 'paragraph');
  paragraphNode.set('align', defaultValues.textAlign);
  paragraphNode.set('line_height', lineHeight);
  paragraphNode.set('parent', ROOT_ID);
  paragraphNode.setContainer('children', new LoroList());

  return doc;
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

export const getLoroDocCharacterCount = (text: string) => {
  return [...text.replaceAll('\u200B', '').replaceAll(/\s+/g, ' ').trim()].length;
};

export const extractLoroDocContents = async (doc: LoroDoc) => {
  const snapshot = new Uint8Array(doc.export({ mode: 'snapshot' }));
  const json = await wasm.snapshotToJson(snapshot);
  const text = extractTextFromLoroDoc(doc);
  const characterCount = getLoroDocCharacterCount(text);
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
