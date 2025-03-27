import { Node } from '@tiptap/pm/model';
import { prosemirrorToYXmlFragment } from 'y-prosemirror';
import * as Y from 'yjs';
import { schema } from '@/pm';
import type { JSONContent } from '@tiptap/core';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  content: JSONContent;
};
export const makeYDoc = ({ title, subtitle, content }: MakeYDocParams) => {
  const node = Node.fromJSON(schema, content);
  const doc = new Y.Doc();

  doc.getText('title').insert(0, title ?? '');
  doc.getText('subtitle').insert(0, subtitle ?? '');

  const fragment = doc.getXmlFragment('content');
  prosemirrorToYXmlFragment(node, fragment);

  return doc;
};
