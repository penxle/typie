import { Extension } from '@tiptap/core';
import { NodeSelection, TextSelection } from '@tiptap/pm/state';
import { findNodeUpward } from '../lib/node-utils';
import { Blockquote, Callout, CodeBlock, Fold, HtmlBlock } from '../node-views';

// NOTE: lift, unwrap 가능한 defining: true인 노드들 (list_item 제외)
export const WRAPPING_NODE_NAMES = [Blockquote.name, Callout.name, Fold.name];

// NOTE: content: text* 인 노드들
export const TEXT_NODE_TYPES = [CodeBlock.name, HtmlBlock.name];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    nodeCommands: {
      unwrapNode: (nodeType: string) => ReturnType;
      selectUpwardNode: (nodeType: string) => ReturnType;
      selectNodeBackwardByTypes: (nodeTypes: string[]) => ReturnType;
      convertNodeToParagraphAtStart: (nodeTypes: string[]) => ReturnType;
      insertNodeWithSelection: (nodeType: string) => ReturnType;
    };
  }
}

export const NodeCommands = Extension.create({
  name: 'nodeCommands',

  addCommands() {
    return {
      unwrapNode:
        (nodeType: string) =>
        ({ state, commands }) => {
          const result = findNodeUpward(state.selection, ({ node, depth }) => {
            if (depth === 0 || node.type.name === 'doc') return false;

            if (node.type.name !== nodeType) return false;
            return true;
          });

          if (result) {
            return commands.lift(result.node.type.name);
          }

          return false;
        },

      selectUpwardNode:
        (nodeType: string) =>
        ({ state, tr, dispatch }) => {
          const { selection } = state;

          const result = findNodeUpward(selection, ({ node, depth }) => {
            if (depth === 0 || node.type.name === 'doc') return false;

            if (node.type.name !== nodeType) return false;

            return true;
          });

          if (result && dispatch) {
            tr.setSelection(NodeSelection.create(state.doc, result.pos));
            return true;
          }

          return false;
        },

      selectNodeBackwardByTypes:
        (nodeTypes: string[]) =>
        ({ state, dispatch }) => {
          const { selection, doc } = state;
          const { $anchor } = selection;

          if (selection.empty && $anchor.pos > 1) {
            const nodeBefore = doc.resolve($anchor.pos - 1).nodeBefore;

            if (nodeBefore && nodeTypes.includes(nodeBefore.type.name)) {
              const nodePos = $anchor.pos - nodeBefore.nodeSize - 1;
              if (dispatch) {
                const tr = state.tr.setSelection(NodeSelection.create(doc, nodePos));
                dispatch(tr);
              }
              return true;
            }
          }

          return false;
        },

      convertNodeToParagraphAtStart:
        (nodeTypes: string[]) =>
        ({ state, commands }) => {
          const { selection } = state;
          const { $anchor } = selection;

          if (selection.empty && nodeTypes.includes($anchor.parent.type.name) && $anchor.parentOffset === 0) {
            return commands.setNode('paragraph');
          }

          return false;
        },

      insertNodeWithSelection:
        (nodeType: string) =>
        ({ state, chain }) => {
          const { selection } = state;
          const { $from, $to } = selection;

          const selectedText = state.doc.textBetween($from.pos, $to.pos, '\n', (node) => {
            if (node.type.name === 'hard_break') {
              return '\n';
            }
            return '';
          });
          if (selection.empty || !selectedText) return false;

          const node = state.schema.nodes[nodeType];

          if (!node || !node.spec.content?.includes('text')) {
            // NOTE: implementation error지만 무시
            return false;
          }

          return chain()
            .deleteSelection()
            .insertNode(node.create(null, state.schema.text(selectedText)))
            .command(({ tr, state }) => {
              const { $from } = state.selection;
              const pos = $from.pos + 1;
              tr.setSelection(TextSelection.create(state.doc, pos, pos + selectedText.length));
              return true;
            })
            .run();
        },
    };
  },
});
