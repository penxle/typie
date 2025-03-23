import { createNodeView } from '$lib/tiptap/lib';
import { values } from '$lib/tiptap/values';
import Component from './Component.svelte';

const blockquotes = values.blockquote.map(({ type }) => type);
type Blockquote = (typeof blockquotes)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    blockquote: {
      setBlockquote: (type?: Blockquote) => ReturnType;
    };
  }
}

export const Blockquote = createNodeView(Component, {
  name: 'blockquote',
  group: 'block',
  content: 'paragraph+',
  defining: true,

  addAttributes() {
    return {
      type: {
        isRequired: true,
        default: blockquotes[0],
        parseHTML: (element) => {
          const blockquote = element.dataset.type;

          if (blockquote && (blockquotes as string[]).includes(blockquote)) {
            return blockquote;
          }

          return blockquotes[0];
        },
        renderHTML: ({ type }) => {
          return {
            'data-type': type,
          };
        },
      },
    };
  },

  addCommands() {
    return {
      setBlockquote:
        (type) =>
        ({ commands }) => {
          return commands.insertContent({ type: this.name, attrs: { type }, content: [{ type: 'paragraph' }] });
        },
    };
  },
});
