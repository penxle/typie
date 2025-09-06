import { Extension, getMarkAttributes } from '@tiptap/core';
import { defaultValues, values } from '../values';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontWeight: {
      setFontWeight: (fontWeight: number) => ReturnType;
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
              if (!fontWeight) {
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
        ({ chain }) => {
          if (fontWeight === defaultValues.fontWeight) {
            return chain().unsetMark('bold').setTextStyle({ fontWeight: null }).run();
          } else {
            return chain().unsetMark('bold').setTextStyle({ fontWeight }).run();
          }
        },

      toggleBold:
        () =>
        ({ commands, state }) => {
          const { fontFamily, fontWeight } = getMarkAttributes(state, 'text_style');

          const weights = values.fontFamily.find((f) => f.value === fontFamily)?.weights || [400, 700];
          const findClosestWeight = (target: number) => {
            return weights.reduce((prev, curr) => {
              return Math.abs(curr - target) < Math.abs(prev - target) ? curr : prev;
            });
          };

          const normalWeight = findClosestWeight(400);
          const boldWeight = findClosestWeight(700);

          if (normalWeight === boldWeight) {
            return false;
          }

          if (!fontWeight || fontWeight < boldWeight) {
            return commands.setFontWeight(boldWeight);
          }

          return commands.setFontWeight(normalWeight);
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-b': () => this.editor.commands.toggleBold(),
    };
  },
});
