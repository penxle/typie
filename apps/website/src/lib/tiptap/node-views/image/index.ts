import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    image: {
      setImage: () => ReturnType;
      insertImageAt: (pos: number) => ReturnType;
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
    };
  },

  addCommands() {
    return {
      setImage:
        () =>
        ({ can, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return commands.insertContent({ type: this.name });
        },
      insertImageAt:
        (pos) =>
        ({ commands }) => {
          return commands.insertContentAt(pos, { type: this.name });
        },
    };
  },
});
