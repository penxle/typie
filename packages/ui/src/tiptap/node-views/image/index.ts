import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    image: {
      setImage: () => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    uploadBlobAsImage: (file: File) => Promise<Record<string, unknown>>;
  }
}

export const Image = createNodeView<unknown, Storage>(Component, {
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
        ({ can, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return commands.insertNode(this.type.create());
        },
    };
  },
});
