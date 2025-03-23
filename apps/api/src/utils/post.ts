import * as Y from 'yjs';
import type { JSONContent } from '@tiptap/core';

type MakeYDocParams = {
  title?: string | null;
  subtitle?: string | null;
  content: JSONContent;
};
export const makeYDoc = ({ title, subtitle }: MakeYDocParams) => {
  const doc = new Y.Doc();

  doc.getText('title').insert(0, title ?? '');
  doc.getText('subtitle').insert(0, subtitle ?? '');

  doc.getXmlFragment('content');
  // prosemirrorJSONToYXmlFragment(content, fragment);

  return doc;
};
