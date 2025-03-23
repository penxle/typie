import { mergeAttributes, Node } from '@tiptap/core';
import { defaultValues, values } from '../values';

const paragraphIndents = values.paragraphIndent.map(({ value }) => value);
type ParagraphIndent = (typeof paragraphIndents)[number];

const maxWidths = values.maxWidth.map(({ value }) => value);
type MaxWidth = (typeof maxWidths)[number];

const blockGaps = values.blockGap.map(({ value }) => value);
type BlockGap = (typeof blockGaps)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    body: {
      setBodyParagraphIndent: (paragraphIndent: ParagraphIndent) => ReturnType;
      setBodyMaxWidth: (maxWidth: MaxWidth) => ReturnType;
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
        default: defaultValues.paragraphIndent,
        renderHTML: ({ paragraphIndent }) => ({
          style: `--prosemirror-paragraph-indent: ${paragraphIndent}rem`,
        }),
      },

      maxWidth: {
        default: defaultValues.maxWidth,
        renderHTML: ({ maxWidth }) => ({
          style: `--prosemirror-max-width: ${maxWidth}px`,
        }),
      },

      blockGap: {
        default: defaultValues.blockGap,
        renderHTML: ({ blockGap }) => ({
          style: `--prosemirror-block-gap: ${blockGap}rem`,
        }),
      },
    };
  },

  renderHTML({ HTMLAttributes }) {
    return ['div', mergeAttributes(HTMLAttributes, { class: 'prose' }), 0];
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

      setBodyMaxWidth:
        (maxWidth) =>
        ({ commands }) => {
          if (!maxWidths.includes(maxWidth)) {
            return false;
          }

          return commands.updateAttributes(this.name, { maxWidth });
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
