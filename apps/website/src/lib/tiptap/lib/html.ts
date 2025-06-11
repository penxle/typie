import { getSchema } from '@tiptap/core';
import { DOMSerializer, Node as ProseMirrorNode } from '@tiptap/pm/model';
import { base64url } from 'rfc4648';
import { handleHTML, parseHTML } from 'zeed-dom';
import type { Extensions, JSONContent } from '@tiptap/core';
import type { Node, Window } from 'happy-dom';

function generateHTML(doc: JSONContent, extensions: Extensions): string {
  const schema = getSchema(extensions);
  const node = ProseMirrorNode.fromJSON(schema, doc);

  const w: Window = typeof window === 'undefined' ? new __happydom__.Window() : globalThis.window;

  const fragment = DOMSerializer.fromSchema(schema).serializeFragment(node.content, {
    document: w.document as unknown as Document,
  });

  const serializer = new w.XMLSerializer();
  return serializer.serializeToString(fragment as unknown as Node);
}

export const renderHTML = (content: JSONContent, extensions: Extensions) => {
  const html = generateHTML(content, extensions);

  let head = '';

  const body = handleHTML(html, (document) => {
    const nodeViewWrappers = document.querySelectorAll('node-view');

    for (const nodeViewWrapper of nodeViewWrappers) {
      head += decode(nodeViewWrapper.dataset.head ?? '');

      const nodeViewContent = parseHTML(decode(nodeViewWrapper.dataset.html ?? ''));
      const nodeView = nodeViewContent.querySelector('[data-node-view]');

      if (!nodeView) {
        continue;
      }

      const nodeViewContentEditableWrapper = nodeViewWrapper.querySelector('node-view-content-editable');
      const nodeViewContentEditable = nodeView.querySelector('[data-node-view-content-editable]');

      if (nodeViewContentEditableWrapper && nodeViewContentEditable) {
        nodeViewContentEditable.append(nodeViewContentEditableWrapper.children);
      }

      nodeViewWrapper.replaceWith(nodeView);
    }
  });

  return { head, body };
};

const decoder = new TextDecoder();
const decode = (value: string) => {
  return decoder.decode(base64url.parse(value));
};
