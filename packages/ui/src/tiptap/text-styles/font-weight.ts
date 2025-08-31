import { Extension, getMarkAttributes } from '@tiptap/core';
import { defaultValues, values } from '../values';

const fontWeights = values.fontWeight.map(({ value }) => value);
type FontWeight = (typeof fontWeights)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontWeight: {
      setFontWeight: (fontWeight: FontWeight) => ReturnType;
      toggleBold: () => ReturnType;
    };
  }
}

export const FontWeight = Extension.create({
  name: 'font_weight',

  addGlobalAttributes() {
    return [
      {
        types: ['text_style'],
        attributes: {
          fontWeight: {
            parseHTML: (element) => {
              const fontWeight = Number(element.style.fontWeight);
              if (!fontWeight || !(fontWeights as number[]).includes(fontWeight)) {
                return null;
              }

              return fontWeight;
            },
            renderHTML: ({ fontWeight }) => {
              if (!fontWeight) {
                return null;
              }

              return {
                style: `font-weight: ${fontWeight}`,
              };
            },
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setFontWeight:
        (fontWeight) =>
        ({ commands }) => {
          if (!fontWeights.includes(fontWeight)) {
            return false;
          }

          if (fontWeight === defaultValues.fontWeight) {
            return commands.setTextStyle({ fontWeight: null });
          } else {
            return commands.setTextStyle({ fontWeight });
          }
        },

      toggleBold:
        () =>
        ({ commands, state }) => {
          const attributes = getMarkAttributes(state, 'text_style');
          const fontWeight = attributes?.fontWeight;
          if (!fontWeight) {
            return commands.setFontWeight(700);
          }

          if (fontWeight >= 700) {
            return commands.setFontWeight(400);
          }

          return commands.setFontWeight(700);
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-b': () => this.editor.commands.toggleBold(),
    };
  },
});
