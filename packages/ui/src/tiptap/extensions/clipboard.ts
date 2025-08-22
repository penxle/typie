import { Extension } from '@tiptap/core';
import { Fragment, Slice } from '@tiptap/pm/model';
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
    return [
      new Plugin({
        props: {
          clipboardTextParser: (text, context, __, view) => {
            const { state } = view;
            const { selection, schema } = state;

            const marks = state.storedMarks || selection.$head.marks();
            const lines = text.split(/(?:\r\n|\n|\r)/g);

            if (lines.length === 1) {
              let textNode = schema.text(lines[0]);
              if (marks.length > 0) {
                textNode = textNode.mark(marks);
              }

              return new Slice(Fragment.from(textNode), 0, 0);
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
                return true;
              }

              let html = event.clipboardData?.getData('text/html');
              if (html) {
                const isGoogleDocs = event.clipboardData?.types.includes('application/x-vnd.google-docs-document-slice-clip+wrapped');

                if (isGoogleDocs) {
                  html = html.replaceAll(/<meta[^>]*>/g, '').replace(/<b\s+[^>]*id="docs-internal-guid[^"]*"[^>]*>(.*?)<\/b>/s, '$1');
                }

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
