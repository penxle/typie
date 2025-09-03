import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    embed: {
      setEmbed: (attrs?: Record<string, unknown>) => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    unfurlEmbed: (url: string) => Promise<Record<string, unknown>>;
  }
}

export const Embed = createNodeView<unknown, Storage>(Component, {
  name: 'embed',
  group: 'block',
  draggable: true,

  addAttributes() {
    return {
      id: {},
      url: {},
      title: {},
      description: {},
      thumbnailUrl: {},
      proportion: { default: 1 },
      html: {},
    };
  },

  addCommands() {
    return {
      setEmbed:
        (attrs) =>
        ({ can, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return commands.insertNode(this.type.create(attrs));
        },
    };
  },
});
