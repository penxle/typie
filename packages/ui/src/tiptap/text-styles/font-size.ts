import { Extension } from '@tiptap/core';
import { closest } from '../../utils';
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

const pattern = /^(\d*\.?\d+)rem$/;

export const FontSize = Extension.create({
  name: 'font_size',

  addGlobalAttributes() {
    return [
      {
        types: ['text_style'],
        attributes: {
          fontSize: {
            parseHTML: (element) => {
              const match = element.style.fontSize.match(pattern);
              if (!match) {
                return null;
              }

              const fontSize = Number.parseFloat(match[1]) * 16;
              return closest(fontSize, fontSizes);
            },
            renderHTML: ({ fontSize }) => {
              if (!fontSize) {
                return null;
              }

              return {
                style: `font-size: ${fontSize / 16}rem`,
              };
            },
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setFontSize:
        (fontSize) =>
        ({ commands }) => {
          if (fontSize === defaultValues.fontSize) {
            return commands.setTextStyle({ fontSize: null });
          } else {
            return commands.setTextStyle({ fontSize: closest(fontSize, fontSizes) });
          }
        },
    };
  },
});
