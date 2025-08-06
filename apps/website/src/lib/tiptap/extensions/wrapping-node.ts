import { Extension } from '@tiptap/core';
import { NodeSelection } from '@tiptap/pm/state';
import { findNodeUpward } from '../lib/node-utils';
import { Blockquote, Callout, Fold } from '../node-views';

export const WRAPPING_NODE_NAMES = [Blockquote.name, Callout.name, Fold.name];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    wrappingNode: {
      unwrapNode: (nodeType?: string) => ReturnType;
      selectUpwardNode: (nodeType?: string) => ReturnType;
    };
  }
}

export const WrappingNode = Extension.create({
  name: 'wrappingNode',

  addCommands() {
    return {
      unwrapNode:
        (nodeType?: string) =>
        ({ state, commands }) => {
          const result = findNodeUpward(state.selection, ({ node, depth }) => {
            if (depth === 0 || node.type.name === 'doc') return false;

            if (nodeType && node.type.name !== nodeType) return false;
            return true;
          });

          if (result) {
            return commands.lift(result.node.type.name);
          }

          return false;
        },

      selectUpwardNode:
        (nodeType?: string) =>
        ({ state, tr, dispatch }) => {
          const { selection } = state;

          const result = findNodeUpward(selection, ({ node, depth }) => {
            if (depth === 0 || node.type.name === 'doc') return false;

            if (nodeType && node.type.name !== nodeType) return false;

            return true;
          });

          if (result && dispatch) {
            tr.setSelection(NodeSelection.create(state.doc, result.pos));
            return true;
          }

          return false;
        },
    };
  },
});
