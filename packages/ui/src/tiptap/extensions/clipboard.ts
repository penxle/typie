import { Extension } from '@tiptap/core';
import { Fragment, Mark, Slice } from '@tiptap/pm/model';
import { Plugin } from '@tiptap/pm/state';
import { handleHTML } from 'zeed-dom';
import { findNodeUpward } from '../lib/node-utils';
import { WRAPPING_NODE_TYPES } from './node-commands';
import type { Selection } from '@tiptap/pm/state';
import type { EditorView } from '@tiptap/pm/view';

export const getWrappingNodeId = (selection: Selection) => {
  const { $from, $to } = selection;

  const result = findNodeUpward(selection, ({ node, depth }) => {
    if (WRAPPING_NODE_TYPES.includes(node.type.name)) {
      const nodeStart = $from.before(depth);
      const nodeEnd = $from.after(depth);

      return $from.pos > nodeStart && $to.pos < nodeEnd;
    }
    return false;
  });

  return result?.node.attrs.nodeId || null;
};

export const getAncestorWrappingNodeIds = (selection: Selection): Set<string> => {
  const { $from, $to } = selection;
  const nodeIds = new Set<string>();

  for (let depth = 1; depth <= $from.depth; depth++) {
    const node = $from.node(depth);
    if (WRAPPING_NODE_TYPES.includes(node.type.name)) {
      const nodeStart = $from.before(depth);
      const nodeEnd = $from.after(depth);

      if ($from.pos > nodeStart && $to.pos < nodeEnd) {
        nodeIds.add(node.attrs.nodeId);
      }
    }
  }

  return nodeIds;
};

export const unwrapNodeById = (fragment: Fragment, nodeId: string): Fragment => {
  const unwrappedNodes = fragment.content.flatMap((node) => {
    if (WRAPPING_NODE_TYPES.includes(node.type.name) && node.attrs.nodeId === nodeId) {
      return node.content.content;
    }

    if (node.content.size === 0) {
      return [node];
    }

    return [node.copy(unwrapNodeById(node.content, nodeId))];
  });

  return Fragment.from(unwrappedNodes);
};

export const unwrapWrappingNodes = (fragment: Fragment, nodeIds: Set<string>): Fragment => {
  const unwrappedNodes = fragment.content.flatMap((node) => {
    if (WRAPPING_NODE_TYPES.includes(node.type.name) && nodeIds.has(node.attrs.nodeId)) {
      return unwrapWrappingNodes(Fragment.from(node.content.content), nodeIds).content;
    }

    if (node.content.size > 0) {
      return [node.copy(unwrapWrappingNodes(node.content, nodeIds))];
    }

    return [node];
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
    const editor = this.editor;

    return [
      new Plugin({
        props: {
          transformPasted: (slice, view) => {
            const { state } = view;
            const { $from } = state.selection;

            const isEmptyParagraph = $from.parent.type.name === 'paragraph' && $from.parent.content.size === 0;
            if (isEmptyParagraph) {
              return new Slice(slice.content, slice.openStart - 1, slice.openEnd);
            }

            return slice;
          },
          clipboardTextParser: (text, _, __, view) => {
            const { state } = view;
            const { selection, schema } = state;

            let marks: readonly Mark[] = [];

            if (selection.empty) {
              marks = state.storedMarks || selection.$head.marks();
            } else {
              state.doc.nodesBetween(selection.from, selection.to, (node) => {
                if (node.isText) {
                  for (const mark of node.marks) {
                    marks = mark.addToSet(marks);
                  }
                }
              });
            }

            let lines = text.split(/(?:\r\n|\n|\r)/g);

            while (lines.length > 0 && lines[0] === '') {
              lines = lines.slice(1);
            }

            while (lines.length > 0 && lines.at(-1) === '') {
              lines = lines.slice(0, -1);
            }

            const $pos = selection.$head;
            let currentBlockNode = $pos.node($pos.depth);

            for (let d = $pos.depth; d >= 0; d--) {
              const node = $pos.node(d);
              if (node.type === schema.nodes.paragraph) {
                currentBlockNode = node;
                break;
              }
            }

            const nodes = [];

            for (const line of lines) {
              let content = Fragment.empty;

              if (line) {
                let textNode = schema.text(line);
                if (marks.length > 0) {
                  textNode = textNode.mark(marks);
                }

                content = Fragment.from(textNode);
              }

              const newParagraph = schema.nodes.paragraph.create(currentBlockNode.attrs, content);
              nodes.push(newParagraph);
            }

            return new Slice(Fragment.from(nodes), 0, 0);
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
              const pmHtml = event.clipboardData?.getData('application/x-pm-html');
              if (pmHtml) {
                event.preventDefault();
                view.pasteHTML(pmHtml, event);
                if (editor.storage.page.layout) {
                  editor.commands.convertIncompatibleBlocks();
                }
                return true;
              }

              const html = event.clipboardData?.getData('text/html');
              if (html) {
                event.preventDefault();
                view.pasteHTML(html, event);
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
