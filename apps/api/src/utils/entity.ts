import { faker } from '@faker-js/faker';
import { getText } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { generateJitteredKeyBetween, indexCharacterSet } from 'fractional-indexing-jittered';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { schema, textSerializers } from '@/pm';
import type { JSONContent } from '@tiptap/core';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  maxWidth?: number;
  body: JSONContent;
  storedMarks?: unknown[];
  note?: string;
  anchors?: Record<string, string | null>;
};
export const makeYDoc = ({ title, subtitle, maxWidth, body, note, storedMarks, anchors }: MakeYDocParams) => {
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
  shapes: Record<string, unknown>;
  orders: string[];
};
export const makeCanvasYDoc = ({ title, shapes, orders }: MakeCanvasYDocParams) => {
  const doc = new Y.Doc();

  doc.transact(() => {
    const attrs = doc.getMap('attrs');
    attrs.set('title', title ?? '');

    const shapesMap = doc.getMap('shapes');
    for (const [id, shape] of Object.entries(shapes)) {
      shapesMap.set(id, shape);
    }

    const ordersArray = doc.getArray('orders');
    for (const order of orders) {
      ordersArray.push([order]);
    }
  });

  return doc;
};

const charSet = indexCharacterSet({ chars: 'ABCDEFGHIJKLMNOPQRSTUVWXYZ' });
type GenerateEntityOrderParams = { lower: string | null | undefined; upper: string | null | undefined };
export const generateEntityOrder = ({ lower, upper }: GenerateEntityOrderParams) => {
  return generateJitteredKeyBetween(lower ?? null, upper ?? null, charSet);
};

export const generateSlug = () => faker.string.hexadecimal({ length: 32, casing: 'lower', prefix: '' });
export const generatePermalink = () => faker.string.alphanumeric({ length: 6, casing: 'mixed' });
