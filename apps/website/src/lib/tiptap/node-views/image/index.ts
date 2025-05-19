import { findChildrenInRange } from '@tiptap/core';
import { NodeSelection } from '@tiptap/pm/state';
import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    image: {
      setImage: () => ReturnType;
    };
  }
}

export const Image = createNodeView(Component, {
  name: 'image',
  group: 'block',
  draggable: true,

  addAttributes() {
    return {
      id: {},
      url: {},
      ratio: {},
      placeholder: {},
      proportion: { default: 1 },
      size: {},
    };
  },

  addCommands() {
    return {
      setImage:
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
