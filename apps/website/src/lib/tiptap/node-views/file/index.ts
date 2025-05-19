import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    file: {
      setFile: (file?: File | { name: string; size: number }) => ReturnType;
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
