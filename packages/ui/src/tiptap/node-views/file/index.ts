import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    file: {
      setFile: () => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    uploadBlobAsFile: (file: File) => Promise<Record<string, unknown>>;
  }
}

export const File = createNodeView<unknown, Storage>(Component, {
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
        ({ can, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return commands.insertNode(this.type.create());
        },
    };
  },
});
