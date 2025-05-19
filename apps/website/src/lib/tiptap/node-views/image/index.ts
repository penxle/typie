import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    image: {
      setImage: (file?: File | string) => ReturnType;
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
        (file) =>
        ({ can, chain }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          let cmd = chain().insertContent({ type: this.name });

          if (file) {
            cmd = cmd.updateNodeViewExtras({ file });
          }

          return cmd.run();
        },
    };
  },
});
