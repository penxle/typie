import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    embed: {
      setEmbed: () => ReturnType;
    };
  }
}

export const Embed = createNodeView(Component, {
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
      html: {},
    };
  },

  addCommands() {
    return {
      setEmbed:
        () =>
        ({ commands }) => {
          return commands.insertContent({ type: 'embed' });
        },
    };
  },
});
