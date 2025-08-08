import { Extension } from '@tiptap/core';
import { DOMParser, Fragment, Slice } from '@tiptap/pm/model';
import { Plugin } from '@tiptap/pm/state';
import { handleHTML } from 'zeed-dom';
import { findNodeUpward } from '../lib/node-utils';
import { WRAPPING_NODE_NAMES } from './node-commands';
import type { Selection } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

const getWrappingNodeId = (selection: Selection) => {
  const { $from, $to } = selection;

  const result = findNodeUpward(selection, ({ node, depth }) => {
    if (WRAPPING_NODE_NAMES.includes(node.type.name)) {
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
    if (WRAPPING_NODE_NAMES.includes(node.type.name) && node.attrs.nodeId === nodeId) {
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
  let slice = selection.content();

  const wrappingNodeId = getWrappingNodeId(selection);
  if (wrappingNodeId) {
    const unwrappedFragment = unwrapNodeById(slice.content, wrappingNodeId);
    slice = new Slice(unwrappedFragment, slice.openStart, slice.openEnd);
  }

  const { dom, text } = view.serializeForClipboard(slice);

  const html = handleHTML(dom.innerHTML, (document) => {
    const body = document.querySelector('.ProseMirror-body');
    if (body) {
      body.replaceWith(...body.children);
    }

    const paragraphs = document.querySelectorAll('p');

    for (const paragraph of paragraphs) {
      if (!paragraph.textContent) {
        paragraph.replaceChildren(document.createElement('br'));
      }
    }
  });

  event.clipboardData?.clearData();
  event.clipboardData?.setData('text/html', html);
  event.clipboardData?.setData('text/plain', text);
  event.clipboardData?.setData('application/x-pm-html', dom.innerHTML);
};

export const Clipboard = Extension.create({
  name: 'clipboard',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          clipboardTextParser: (text, context, __, view) => {
            const parser = DOMParser.fromSchema(view.state.schema);
            const dom = document.createElement('div');

            for (const line of text.split('\n')) {
              const p = document.createElement('p');
              p.textContent = line;
              dom.append(p);
            }

            return parser.parseSlice(dom, {
              context,
              preserveWhitespace: 'full',
            });
          },

          clipboardTextSerializer: (content) => {
            const text = content.content.textBetween(0, content.content.size, '\n', (node) => {
              if (node.type.name === 'hard_break') {
                return '\n';
              }

              return '';
            });

            return text;
          },

          handleDOMEvents: {
            cut: (view, event) => {
              event.preventDefault();

              copy(view, event);

              const { tr } = view.state;
              tr.deleteSelection();
              view.dispatch(tr);

              return true;
            },
            copy: (view, event) => {
              event.preventDefault();

              copy(view, event);

              return true;
            },
            paste: (view, event) => {
              const html = event.clipboardData?.getData('application/x-pm-html');
              if (html) {
                event.preventDefault();

                view.pasteHTML(html);

                return true;
              }

              return false;
            },
          },
        },
      }),
    ];
  },
});
