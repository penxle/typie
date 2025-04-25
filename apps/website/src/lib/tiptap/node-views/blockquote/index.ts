import { createNodeView } from '../../lib';
import { defaultValues, values } from '../../values';
import Component from './Component.svelte';

const blockquotes = values.blockquote.map(({ type }) => type);
type Blockquote = (typeof blockquotes)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    blockquote: {
      toggleBlockquote: (type?: Blockquote) => ReturnType;
      insertBlockquoteAt: (pos: number, type?: Blockquote) => ReturnType;
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
      toggleBlockquote:
        (type) =>
        ({ editor, commands }) => {
          if (editor.isActive(this.name, { type })) {
            return commands.lift(this.name);
          } else if (editor.isActive(this.name)) {
            return commands.updateAttributes(this.name, { type });
          } else {
            return commands.wrapIn(this.name, { type });
          }
        },
      insertBlockquoteAt:
        (pos, type) =>
        ({ chain }) => {
          return chain().insertContentAt(pos, { type: 'paragraph' }).focus().wrapIn(this.name, { type }).run();
        },
    };
  },
});
