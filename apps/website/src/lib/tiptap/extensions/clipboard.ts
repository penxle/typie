import { Extension } from '@tiptap/core';
import { Fragment, Slice } from '@tiptap/pm/model';
import { Plugin } from '@tiptap/pm/state';
import { Blockquote, Fold } from '../node-views';
import type { Node } from '@tiptap/pm/model';
import type { Selection } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

const WRAPPING_NODE_NAMES = new Set([Fold.name, Blockquote.name]);

const getWrappingNodeId = (selection: Selection) => {
  const { $from, $to } = selection;

  for (let depth = $from.depth; depth >= 0; depth--) {
    const node = $from.node(depth);
    if (WRAPPING_NODE_NAMES.has(node.type.name)) {
      const nodeStart = $from.before(depth);
      const nodeEnd = $from.after(depth);

      if ($from.pos > nodeStart && $to.pos < nodeEnd) {
        return node.attrs.nodeId;
      }
    }
  }

  return null;
};

const unwrapNodes = (fragment: Fragment, nodeId: string): Fragment => {
  const nodes: Node[] = [];

  fragment.forEach((node) => {
    if (WRAPPING_NODE_NAMES.has(node.type.name) && node.attrs.nodeId === nodeId) {
      const unwrappedContent = node.content;
      unwrappedContent.forEach((child) => {
        nodes.push(child);
      });
    } else if (node.content.size > 0) {
      const unwrappedContent = unwrapNodes(node.content, nodeId);
      nodes.push(node.copy(unwrappedContent));
    } else {
      nodes.push(node);
    }
  });

  return Fragment.from(nodes);
};

const copy = (view: EditorView, event: ClipboardEvent) => {
  const { selection } = view.state;

  event.preventDefault();

  let slice = selection.content();

  const wrappingNodeId = getWrappingNodeId(selection);
  if (wrappingNodeId) {
    const unwrappedFragment = unwrapNodes(slice.content, wrappingNodeId);
    slice = new Slice(unwrappedFragment, slice.openStart, slice.openEnd);
  }

  const { dom, text } = view.serializeForClipboard(slice);

  event.clipboardData?.clearData();
  event.clipboardData?.setData('text/html', dom.innerHTML);
  event.clipboardData?.setData('text/plain', text);

  return true;
};

export const Clipboard = Extension.create({
  name: 'clipboard',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleDOMEvents: {
            cut: (view, event) => {
              copy(view, event);

              const { tr } = view.state;
              tr.deleteSelection();
              view.dispatch(tr);

              return true;
            },
            copy: (view, event) => copy(view, event),
          },
        },
      }),
    ];
  },
});
