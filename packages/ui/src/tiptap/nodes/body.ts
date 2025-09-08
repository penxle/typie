import { mergeAttributes, Node } from '@tiptap/core';
import { Mark } from '@tiptap/pm/model';
import { Plugin } from '@tiptap/pm/state';
import { css, cx } from '@typie/styled-system/css';
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
      loadTemplate: (post: { body: Record<string, unknown>; storedMarks: Mark[] }) => ReturnType;
    };
  }
}

export const Body = Node.create({
  name: 'body',
  content: 'block+',
  isolating: true,

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
      mergeAttributes(HTMLAttributes, {
        class: cx(
          this.editor?.isEditable ? 'ProseMirror-editable ProseMirror-body' : 'ProseMirror-body',
          css({
            paddingTop: 'var(--prosemirror-padding-top)',
            paddingLeft: 'var(--prosemirror-padding-x)',
            paddingRight: 'var(--prosemirror-padding-x)',
            paddingBottom: 'var(--prosemirror-padding-bottom)',
            '[data-layout="page"] &': {
              minWidth: 'var(--prosemirror-max-width)',
              paddingTop: '[calc(var(--prosemirror-page-margin-top) + var(--prosemirror-padding-top))]',
              paddingLeft: '[calc(var(--prosemirror-page-margin-left) + var(--prosemirror-padding-x))]',
              paddingBottom: '[calc(var(--prosemirror-page-margin-bottom) + var(--prosemirror-padding-bottom))]',
              paddingRight: '[calc(var(--prosemirror-page-margin-right) + var(--prosemirror-padding-x))]',
            },
            '& > .paragraph-indent, & > .selected-node > .paragraph-indent': {
              textIndent: 'var(--prosemirror-paragraph-indent)',
            },
            '& > :is(ol, ul), & > .selected-node > :is(ol, ul)': {
              paddingLeft: 'var(--prosemirror-paragraph-indent)',
            },
          }),
        ),
      }),
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

      loadTemplate:
        ({ body, storedMarks }) =>
        ({ chain, state }) => {
          const { schema } = state;

          return chain()
            .command(({ tr }) => {
              tr.setMeta('template', true);
              return true;
            })
            .focus(2)
            .setContent(body)
            .command(({ tr, dispatch }) => {
              if (storedMarks && storedMarks.length > 0) {
                tr.setStoredMarks(storedMarks.map((mark: unknown) => Mark.fromJSON(schema, mark)));
              }
              dispatch?.(tr);
              return true;
            })
            .setTextSelection(2)
            .run();
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        filterTransaction: (tr, state) => {
          const oldBody = state.doc.firstChild;
          const newBody = tr.doc.firstChild;

          if (oldBody?.attrs.nodeId && !newBody?.attrs.nodeId) {
            return false;
          }

          return true;
        },
      }),
    ];
  },
});
