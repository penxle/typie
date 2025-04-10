import { Mark } from '@tiptap/core';
import { closest } from '$lib/utils';
import { defaultValues, values } from '../values';

const fontSizes = values.fontSize.map(({ value }) => value);
type FontSize = (typeof fontSizes)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontSize: {
      setFontSize: (fontSize: FontSize) => ReturnType;
    };
  }
}

export const FontSize = Mark.create({
  name: 'font_size',
  priority: 120,

  addAttributes() {
    return {
      value: {
        parseHTML: (element) => {
          const fontSize = Number.parseFloat(element.style.fontSize.replace(/rem$/, '')) * 16;
          return closest(fontSize, fontSizes);
        },
        renderHTML: ({ value }) => ({
          style: `font-size: ${value / 16}rem`,
        }),
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span',
        getAttrs: (element) => (element as HTMLElement).style.fontSize.endsWith('rem') && null,
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', HTMLAttributes, 0];
  },

  addCommands() {
    return {
      setFontSize:
        (value) =>
        ({ commands, can }) => {
          if (!can().isMarkAllowed(this.name)) {
            return false;
          }

          if (value === defaultValues.fontSize) {
            return commands.unsetMark(this.name);
          } else {
            return commands.setMark(this.name, { value: closest(value, fontSizes) });
          }
        },
    };
  },
});
