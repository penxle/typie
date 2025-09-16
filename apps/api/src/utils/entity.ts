import { faker } from '@faker-js/faker';
import { getText } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { PostLayoutMode } from '@/enums';
import { schema, textSerializers } from '@/pm';
import type { JSONContent } from '@tiptap/core';
import type { CanvasShape } from '@/db/schemas/json';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  maxWidth?: number;
  body: JSONContent;
  storedMarks?: unknown[];
  note?: string;
  anchors?: Record<string, string | null>;
  layoutMode?: PostLayoutMode;
  pageLayout?: unknown;
};
export const makeYDoc = ({ title, subtitle, maxWidth, body, note, storedMarks, anchors, layoutMode, pageLayout }: MakeYDocParams) => {
  const node = Node.fromJSON(schema, body);
  const doc = new Y.Doc();

  doc.transact(() => {
    const map = doc.getMap('attrs');
    map.set('title', title ?? '');
    map.set('subtitle', subtitle ?? '');
    map.set('maxWidth', maxWidth ?? 800);
    map.set('storedMarks', storedMarks ?? []);
    map.set('note', note ?? '');
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

type MakeCanvasYDocParams = {
  title?: string | null;
  shapes: CanvasShape[];
};
export const makeCanvasYDoc = ({ title, shapes }: MakeCanvasYDocParams) => {
  const doc = new Y.Doc();

  doc.transact(() => {
    const attrs = doc.getMap('attrs');
    attrs.set('title', title ?? '');

    const fragment = doc.getXmlFragment('shapes');
    for (const shape of shapes) {
      const element = new Y.XmlElement(shape.type);
      for (const [key, value] of Object.entries(shape.attrs)) {
        if (value !== undefined && value !== null) {
          element.setAttribute(key, JSON.stringify(value));
        }
      }
      fragment.push([element]);
    }
  });

  return doc;
};

export const generateSlug = () => faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' });
export const generatePermalink = () => faker.string.alphanumeric({ length: 6, casing: 'mixed' });
