import { Extension } from '@tiptap/core';
import { Fragment, Slice } from '@tiptap/pm/model';
import { Plugin } from '@tiptap/pm/state';
import { findNodeUpward } from '../lib/node-utils';
import { Blockquote, Callout, Fold } from '../node-views';
import type { Selection } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

const WRAPPING_NODE_NAMES = new Set([Fold.name, Blockquote.name, Callout.name]);

const getWrappingNodeId = (selection: Selection) => {
  const { $from, $to } = selection;

  const result = findNodeUpward(selection, ({ node, depth }) => {
    if (WRAPPING_NODE_NAMES.has(node.type.name)) {
      const nodeStart = $from.before(depth);
      const nodeEnd = $from.after(depth);

      return $from.pos > nodeStart && $to.pos < nodeEnd;
    }
    return false;
  });

  return result?.node.attrs.nodeId || null;
};

const unwrapNodeById = (fragment: Fragment, nodeId: string): Fragment => {
  const unwrappedNodes = fragment.content.flatMap((node) => {
    if (WRAPPING_NODE_NAMES.has(node.type.name) && node.attrs.nodeId === nodeId) {
      return node.content.content;
    }

    if (node.content.size === 0) {
      return [node];
    }

    return [node.copy(unwrapNodeById(node.content, nodeId))];
  });

  return Fragment.from(unwrappedNodes);
};

const copy = (view: EditorView, event: ClipboardEvent) => {
  const { selection } = view.state;

  event.preventDefault();

  let slice = selection.content();

  const wrappingNodeId = getWrappingNodeId(selection);
  if (wrappingNodeId) {
    const unwrappedFragment = unwrapNodeById(slice.content, wrappingNodeId);
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
