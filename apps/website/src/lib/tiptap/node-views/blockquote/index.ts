import { createNodeView } from '$lib/tiptap/lib';
import { defaultValues, values } from '$lib/tiptap/values';
import Component from './Component.svelte';

const blockquotes = values.blockquote.map(({ type }) => type);
type Blockquote = (typeof blockquotes)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    blockquote: {
      setBlockquote: (type?: Blockquote) => ReturnType;
      toggleBlockquote: (type?: Blockquote) => ReturnType;
      unsetBlockquote: () => ReturnType;
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
        default: defaultValues.blockquote,
        parseHTML: (element) => {
          const blockquote = element.dataset.type;

          if (blockquote && (blockquotes as string[]).includes(blockquote)) {
            return blockquote;
          }

          return defaultValues.blockquote;
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
          return commands.wrapIn(this.name, { type });
        },
      toggleBlockquote:
        (type) =>
        ({ commands }) => {
          return commands.toggleWrap(this.name, { type });
        },
      unsetBlockquote:
        () =>
        ({ commands }) => {
          return commands.lift(this.name);
        },
    };
  },
});
