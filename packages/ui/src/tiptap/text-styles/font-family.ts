import { Extension } from '@tiptap/core';
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

export const FontFamily = Extension.create({
  name: 'font_family',

  addGlobalAttributes() {
    return [
      {
        types: ['text_style'],
        attributes: {
          fontFamily: {
            parseHTML: (element) => {
              const fontFamily = element.style.fontFamily;
              if (!fontFamily || (!(fontFamilies as string[]).includes(fontFamily) && !fontFamily.startsWith('FONT0'))) {
                return null;
              }

              return fontFamily;
            },
            renderHTML: ({ fontFamily }) => {
              if (!fontFamily) {
                return null;
              }

              return {
                style: `font-family: ${fontFamily}`,
              };
            },
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setFontFamily:
        (fontFamily) =>
        ({ commands }) => {
          if (!fontFamilies.includes(fontFamily) && !fontFamily.startsWith('FONT0')) {
            return false;
          }

          if (fontFamily === defaultValues.fontFamily) {
            return commands.setTextStyle({ fontFamily: null });
          } else {
            return commands.setTextStyle({ fontFamily });
          }
        },
    };
  },
});
