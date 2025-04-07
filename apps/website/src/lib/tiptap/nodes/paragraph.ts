import { mergeAttributes, Node } from '@tiptap/core';
import { defaultValues, values } from '$lib/tiptap/values';
import { closest } from '$lib/utils';
import { css } from '$styled-system/css';

const textAligns = values.textAlign.map(({ value }) => value);
type TextAlign = (typeof textAligns)[number];

const lineHeights = values.lineHeight.map(({ value }) => value);
type LineHeight = (typeof lineHeights)[number];

const letterSpacings = values.letterSpacing.map(({ value }) => value);
type LetterSpacing = (typeof letterSpacings)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    paragraph: {
      setParagraph: () => ReturnType;
      setParagraphTextAlign: (textAlign: TextAlign) => ReturnType;
      setParagraphLineHeight: (lineHeight: LineHeight) => ReturnType;
      setParagraphLetterSpacing: (letterSpacing: LetterSpacing) => ReturnType;
    };
  }
}

export const Paragraph = Node.create({
  name: 'paragraph',
  group: 'block',
  content: 'inline*',
  priority: 255,

  addAttributes() {
    return {
      textAlign: {
        default: defaultValues.textAlign,
        parseHTML: (element) => {
          const textAlign = element.style.textAlign;
          if (!(textAligns as string[]).includes(textAlign)) {
            return defaultValues.textAlign;
          }

          return textAlign;
        },
        renderHTML: ({ textAlign }) => ({
          style: `text-align: ${textAlign}`,
        }),
      },

      lineHeight: {
        default: defaultValues.lineHeight,
        parseHTML: (element) => {
          const lineHeight = Number.parseFloat(element.style.lineHeight);
          return closest(lineHeight, lineHeights) ?? defaultValues.lineHeight;
        },
        renderHTML: ({ lineHeight }) => ({
          style: `line-height: ${lineHeight}`,
        }),
      },

      letterSpacing: {
        default: defaultValues.letterSpacing,
        parseHTML: (element) => {
          const letterSpacing = Number.parseFloat(element.style.letterSpacing.replace(/em$/, ''));
          return closest(letterSpacing, letterSpacings) ?? defaultValues.letterSpacing;
        },
        renderHTML: ({ letterSpacing }) => ({
          style: `letter-spacing: ${letterSpacing - 0.025}em`,
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'p' }];
  },

  renderHTML({ node, HTMLAttributes }) {
    return [
      'p',
      mergeAttributes(HTMLAttributes, {
        class: css(
          (node.attrs.textAlign === 'left' || node.attrs.textAlign === 'justify') && {
            textIndent: 'var(--prosemirror-paragraph-indent)',
          },
        ),
      }),
      !this.editor?.isEditable && node.content.size === 0 ? ['br'] : 0,
    ];
  },

  addCommands() {
    return {
      setParagraph:
        () =>
        ({ commands }) => {
          return commands.setNode(this.name);
        },

      setParagraphTextAlign:
        (textAlign) =>
        ({ commands }) => {
          if (!textAligns.includes(textAlign)) {
            return false;
          }

          return commands.updateAttributes(this.name, { textAlign });
        },

      setParagraphLineHeight:
        (lineHeight) =>
        ({ commands }) => {
          if (!lineHeights.includes(lineHeight)) {
            return false;
          }

          return commands.updateAttributes(this.name, { lineHeight });
        },

      setParagraphLetterSpacing:
        (letterSpacing) =>
        ({ commands }) => {
          if (!letterSpacings.includes(letterSpacing)) {
            return false;
          }

          return commands.updateAttributes(this.name, { letterSpacing });
        },
    };
  },
});
