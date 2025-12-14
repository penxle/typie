import { faker } from '@faker-js/faker';
import { getText } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { LoroDoc, LoroList, LoroMap } from 'loro-crdt';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
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
  layoutMode.set('page_margin', 96);

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
