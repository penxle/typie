import { getText, getTextSerializersFromSchema } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { schema } from '@/pm';
import type { JSONContent } from '@tiptap/core';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  body: JSONContent;
};
export const makeYDoc = ({ title, subtitle, body }: MakeYDocParams) => {
  const node = Node.fromJSON(schema, body);
  const doc = new Y.Doc();

  const map = doc.getMap('attrs');
  map.set('title', title ?? '');
  map.set('subtitle', subtitle ?? '');
  map.set('maxWidth', 1000);

  const fragment = doc.getXmlFragment('body');
  prosemirrorToYXmlFragment(node, fragment);

  return doc;
};

export const makeText = (body: JSONContent) => {
  const node = Node.fromJSON(schema, body);

  return getText(node, {
    blockSeparator: '\n',
    textSerializers: getTextSerializersFromSchema(schema),
  }).trim();
};
