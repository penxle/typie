import { mergeAttributes, Node } from '@tiptap/core';
import { defaultValues, values } from '../values';

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
        default: defaultValues.paragraphIndent,
        renderHTML: ({ paragraphIndent }) => ({
          style: `--prosemirror-paragraph-indent: ${paragraphIndent}rem`,
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
    return [
      'div',
      mergeAttributes(HTMLAttributes, { class: this.editor?.isEditable ? 'ProseMirror-editable ProseMirror-body' : 'ProseMirror-body' }),
      0,
    ];
  },

  addCommands() {
    return {
      setBodyParagraphIndent:
        (paragraphIndent) =>
        ({ tr, dispatch }) => {
          if (!paragraphIndents.includes(paragraphIndent)) {
            return false;
          }

          tr.setNodeAttribute(0, 'paragraphIndent', paragraphIndent);

          if (dispatch) {
            dispatch(tr);
          }

          return true;
        },

      setBodyBlockGap:
        (blockGap) =>
        ({ tr, dispatch }) => {
          if (!blockGaps.includes(blockGap)) {
            return false;
          }

          tr.setNodeAttribute(0, 'blockGap', blockGap);

          if (dispatch) {
            dispatch(tr);
          }

          return true;
        },
    };
  },
});
