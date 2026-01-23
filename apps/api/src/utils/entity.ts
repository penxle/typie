import { faker } from '@faker-js/faker';
import { getText } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { inArray } from 'drizzle-orm';
import { LoroDoc, LoroList, LoroMap } from 'loro-crdt';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { db, Files, Images } from '@/db';
import { PostLayoutMode } from '@/enums';
import { schema, textSerializers } from '@/pm';
import type { JSONContent } from '@tiptap/core';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  maxWidth?: number;
  body: JSONContent;
  storedMarks?: unknown[];
  initialMarks?: unknown[];
  anchors?: Record<string, string | null>;
  layoutMode?: PostLayoutMode;
  pageLayout?: unknown;
};
export const makeYDoc = ({
  title,
  subtitle,
  maxWidth,
  body,
  storedMarks,
  initialMarks,
  anchors,
  layoutMode,
  pageLayout,
}: MakeYDocParams) => {
  const node = Node.fromJSON(schema, body);
  const doc = new Y.Doc();

  doc.transact(() => {
    const map = doc.getMap('attrs');
    map.set('title', title ?? '');
    map.set('subtitle', subtitle ?? '');
    map.set('maxWidth', maxWidth ?? 800);
    map.set('storedMarks', storedMarks ?? []);
    map.set('initialMarks', initialMarks ?? []);
    map.set('anchors', anchors ?? {});
    map.set('layoutMode', layoutMode ?? PostLayoutMode.SCROLL);
    map.set('pageLayout', pageLayout ?? null);

    const fragment = doc.getXmlFragment('body');
    prosemirrorToYXmlFragment(node, fragment);
  });

  return doc;
};

export const makeText = (body: JSONContent) => {
  const node = Node.fromJSON(schema, body);

  return getText(node, {
    blockSeparator: '\n',
    textSerializers,
  }).trim();
};

export const generateSlug = () => faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' });
export const generatePermalink = () => faker.string.alphanumeric({ length: 6, casing: 'mixed' });

export const makeLoroDoc = () => {
  const doc = new LoroDoc();

  const settings = doc.getMap('settings');
  settings.set('block_gap', 1);
  settings.set('paragraph_indent', 1);

  const layoutMode = settings.setContainer('layout_mode', new LoroMap());
  layoutMode.set('type', 'paginated');
  layoutMode.set('page_width', 794);
  layoutMode.set('page_height', 1123);
  layoutMode.set('page_margin_top', 96);
  layoutMode.set('page_margin_bottom', 96);
  layoutMode.set('page_margin_left', 96);
  layoutMode.set('page_margin_right', 96);

  const ROOT_ID = '00000000000000000000000000000000';
  const paragraphId = faker.string.uuid().replaceAll('-', '');

  const nodes = doc.getMap('nodes');

  const rootNode = nodes.setContainer(ROOT_ID, new LoroMap());
  rootNode.set('type', 'root');
  const rootChildren = rootNode.setContainer('children', new LoroList());
  rootChildren.insert(0, paragraphId);

  const paragraphNode = nodes.setContainer(paragraphId, new LoroMap());
  paragraphNode.set('type', 'paragraph');
  paragraphNode.set('align', 'left');
  paragraphNode.set('line_height', 1.6);
  paragraphNode.set('parent', ROOT_ID);
  paragraphNode.setContainer('children', new LoroList());

  return doc;
};

const extractTextFromLoroDoc = (doc: LoroDoc): string => {
  const nodes = doc.getMap('nodes').toJSON() as Record<string, { text?: string }>;
  const texts: string[] = [];

  for (const node of Object.values(nodes)) {
    if (node.text) {
      texts.push(node.text);
    }
  }

  return texts.join('');
};

export const extractAssetIdsFromLoroDoc = (doc: LoroDoc): { imageIds: string[]; fileIds: string[] } => {
  const nodes = doc.getMap('nodes').toJSON() as Record<string, unknown>;

  const imageIds: string[] = [];
  const fileIds: string[] = [];

  for (const node of Object.values(nodes)) {
    const typedNode = node as { type?: string; id?: string };
    if (typedNode.type === 'image' && typedNode.id) {
      imageIds.push(typedNode.id);
    } else if (typedNode.type === 'file' && typedNode.id) {
      fileIds.push(typedNode.id);
    }
  }

  return { imageIds, fileIds };
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

export const getLoroDocCharacterCount = (text: string) => {
  return [...text.replaceAll('\u200B', '').replaceAll(/\s+/g, ' ').trim()].length;
};

export const extractLoroDocContents = async (doc: LoroDoc) => {
  const json = doc.toJSON();
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
