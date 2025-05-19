import { findChildrenInRange } from '@tiptap/core';
import { NodeSelection } from '@tiptap/pm/state';
import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    file: {
      setFile: () => ReturnType;
    };
  }
}

export const File = createNodeView(Component, {
  name: 'file',
  group: 'block',

  addAttributes() {
    return {
      id: {},
      name: {},
      size: {},
      url: {},
    };
  },

  addCommands() {
    return {
      setFile:
        () =>
        ({ can, tr, dispatch }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          const node = this.type.create();
          tr.replaceSelectionWith(node);

          const children = findChildrenInRange(tr.doc, { from: 0, to: tr.selection.anchor }, (node) => node.type === this.type);
          const pos = children.at(-1)?.pos;

          if (pos) {
            tr.setSelection(NodeSelection.create(tr.doc, pos));
          }

          dispatch?.(tr);

          return true;
        },
    };
  },
});
