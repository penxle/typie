import { mergeAttributes, Node } from '@tiptap/core';
import { values } from '../values';

const paragraphIndents = values.paragraphIndent.map(({ value }) => value);
type ParagraphIndent = (typeof paragraphIndents)[number];

const blockGaps = values.blockGap.map(({ value }) => value);
type BlockGap = (typeof blockGaps)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    body: {
      setBodyParagraphIndent: (paragraphIndent: ParagraphIndent) => ReturnType;
      setBodyBlockGap: (blockGap: BlockGap) => ReturnType;
    };
  }
}

export const Body = Node.create({
  name: 'body',
  content: 'block+',

  addAttributes() {
    return {
      paragraphIndent: {
        default: 1,
        renderHTML: ({ paragraphIndent }) => ({
          style: `--prosemirror-paragraph-indent: ${paragraphIndent}rem`,
        }),
      },

      blockGap: {
        default: 1,
        renderHTML: ({ blockGap }) => ({
          style: `--prosemirror-block-gap: ${blockGap}rem`,
        }),
      },
    };
  },

  renderHTML({ HTMLAttributes }) {
    return ['article', mergeAttributes(HTMLAttributes, { class: 'prose' }), 0];
  },

  addCommands() {
    return {
      setBodyParagraphIndent:
        (paragraphIndent) =>
        ({ commands }) => {
          if (!paragraphIndents.includes(paragraphIndent)) {
            return false;
          }

          return commands.updateAttributes(this.name, { paragraphIndent });
        },

      setBodyBlockGap:
        (blockGap) =>
        ({ commands }) => {
          if (!blockGaps.includes(blockGap)) {
            return false;
          }

          return commands.updateAttributes(this.name, { blockGap });
        },
    };
  },
});
