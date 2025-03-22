import { Node } from '@tiptap/core';
import { values } from '../values';

const globalParagraphIndents = values.globalParagraphIndent.map(({ value }) => value);
type GlobalParagraphIndent = (typeof globalParagraphIndents)[number];

const globalParagraphSpacings = values.globalParagraphSpacing.map(({ value }) => value);
type GlobalParagraphSpacing = (typeof globalParagraphSpacings)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    body: {
      setGlobalParagraphIndent: (globalParagraphIndent: GlobalParagraphIndent) => ReturnType;
      setGlobalParagraphSpacing: (globalParagraphSpacing: GlobalParagraphSpacing) => ReturnType;
    };
  }
}

export const Body = Node.create({
  name: 'body',
  content: 'block+',

  addAttributes() {
    return {
      globalParagraphIndent: {
        default: 1,
        renderHTML: ({ globalParagraphIndent }) => ({
          style: `--global-paragraph-indent: ${globalParagraphIndent}rem`,
        }),
      },

      globalParagraphSpacing: {
        default: 1,
        renderHTML: ({ globalParagraphSpacing }) => ({
          style: `--global-paragraph-spacing: ${globalParagraphSpacing}rem`,
        }),
      },
    };
  },

  renderHTML({ HTMLAttributes }) {
    return ['div', HTMLAttributes, 0];
  },

  addCommands() {
    return {
      setGlobalParagraphIndent:
        (globalParagraphIndent) =>
        ({ commands }) => {
          if (!globalParagraphIndents.includes(globalParagraphIndent)) {
            return false;
          }

          return commands.updateAttributes(this.name, { globalParagraphIndent });
        },

      setGlobalParagraphSpacing:
        (globalParagraphSpacing) =>
        ({ commands }) => {
          if (!globalParagraphSpacings.includes(globalParagraphSpacing)) {
            return false;
          }

          return commands.updateAttributes(this.name, { globalParagraphSpacing });
        },
    };
  },
});
