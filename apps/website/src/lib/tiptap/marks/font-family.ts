import { Mark } from '@tiptap/core';
import { defaultValues, values } from '../values';

const fontFamilies = values.fontFamily.map(({ value }) => value);
type FontFamily = (typeof fontFamilies)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontFamily: {
      setFontFamily: (fontFamily: FontFamily) => ReturnType;
    };
  }
}

export const FontFamily = Mark.create({
  name: 'font_family',
  priority: 120,

  addAttributes() {
    return {
      value: {
        parseHTML: (element) => element.style.fontFamily,
        renderHTML: ({ value }) => ({
          style: `font-family: ${value}`,
        }),
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span',
        getAttrs: (node) => {
          const fontFamily = (node as HTMLElement).style.fontFamily;

          if ((fontFamilies as string[]).includes(fontFamily)) {
            return null;
          }

          return false;
        },
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', HTMLAttributes, 0];
  },

  addCommands() {
    return {
      setFontFamily:
        (value) =>
        ({ commands, can }) => {
          if (!fontFamilies.includes(value)) {
            return false;
          }

          if (!can().isMarkAllowed(this.name)) {
            return false;
          }

          if (value === defaultValues.fontFamily) {
            return commands.unsetMark(this.name);
          } else {
            return commands.setMark(this.name, { value });
          }
        },
    };
  },
});
