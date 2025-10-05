import { Extension } from '@tiptap/core';
import { defaultValues } from '../values';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontSize: {
      setFontSize: (fontSize: number) => ReturnType;
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

              return Number.parseFloat(match[1]) * 16;
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
            return commands.setTextStyle({ fontSize });
          }
        },
    };
  },
});
