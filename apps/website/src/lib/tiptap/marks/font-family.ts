import { Mark } from '@tiptap/core';
import { values } from '$lib/tiptap/values';

const fontFamilies = values.fontFamily.map(({ value }) => value);
type FontFamily = (typeof fontFamilies)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontFamily: {
      setFontFamily: (fontFamily: FontFamily) => ReturnType;
      unsetFontFamily: () => ReturnType;
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
        ({ commands }) => {
          if (!fontFamilies.includes(value)) {
            return false;
          }

          return commands.setMark(this.name, { value });
        },

      unsetFontFamily:
        () =>
        ({ commands }) => {
          return commands.unsetMark(this.name);
        },
    };
  },
});
