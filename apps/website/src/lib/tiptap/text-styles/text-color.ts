import { TinyColor } from '@ctrl/tinycolor';
import { Extension } from '@tiptap/core';
import { css } from '$styled-system/css';
import { defaultValues, values } from '../values';

const hexes = Object.fromEntries(values.textColor.map(({ value, hex }) => [value, hex]));
const textColors = values.textColor.map(({ value }) => value);
type TextColor = (typeof textColors)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    textColor: {
      setTextColor: (textColor: TextColor) => ReturnType;
    };
  }
}

export const TextColor = Extension.create({
  name: 'text_color',

  addGlobalAttributes() {
    return [
      {
        types: ['text_style'],
        attributes: {
          textColor: {
            parseHTML: (element) => {
              const color = new TinyColor(element.style.color);
              if (!color.isValid) {
                return null;
              }

              return normalize(color);
            },
            renderHTML: ({ textColor }) => {
              if (!textColor) {
                return null;
              }

              if (textColor === 'white') {
                return {
                  style: `color: ${hexes[textColor]};`,
                  class: css({
                    '& .selected-text': {
                      color: 'gray.500',
                    },
                  }),
                };
              } else {
                return {
                  style: `color: ${hexes[textColor]};`,
                };
              }
            },
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setTextColor:
        (textColor) =>
        ({ commands }) => {
          if (!textColors.includes(textColor)) {
            return false;
          }

          if (textColor === defaultValues.textColor) {
            return commands.setTextStyle({ textColor: null });
          } else {
            return commands.setTextStyle({ textColor });
          }
        },
    };
  },
});

const normalize = (color: TinyColor) => {
  const input = color.toRgb();

  return textColors.reduce(
    (closest, value) => {
      const target = new TinyColor(hexes[value]).toRgb();
      const d = Math.hypot(input.r - target.r, input.g - target.g, input.b - target.b);
      return d < closest.d ? { value, d } : closest;
    },
    { value: textColors[0], d: Number.MAX_VALUE },
  ).value;
};
