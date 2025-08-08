import { TinyColor } from '@ctrl/tinycolor';
import { Extension } from '@tiptap/core';
import { defaultValues, values } from '../values';

const colors = Object.fromEntries(
  values.textBackgroundColor.filter(({ color }) => color !== null).map(({ value, color }) => [value, color]),
) as Record<TextBackgroundColor, string>;
const hexColors = Object.fromEntries(
  values.textBackgroundColor.filter(({ hex }) => hex !== null).map(({ value, hex }) => [value, hex]),
) as Record<TextBackgroundColor, string>;
const textBackgroundColors = values.textBackgroundColor.map(({ value }) => value);
type TextBackgroundColor = (typeof textBackgroundColors)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    textBackgroundColor: {
      setTextBackgroundColor: (textBackgroundColor: TextBackgroundColor) => ReturnType;
    };
  }
}

export const TextBackgroundColor = Extension.create({
  name: 'text_background_color',

  addGlobalAttributes() {
    return [
      {
        types: ['text_style'],
        attributes: {
          textBackgroundColor: {
            parseHTML: (element) => {
              const value = element.style.backgroundColor;

              const match = value.match(/var\(--colors-prosemirror\\?\\.bg\\?\\.([^)]+)\)/);
              if (match) {
                const name = match[1] as TextBackgroundColor;
                if (textBackgroundColors.includes(name)) {
                  return name;
                }
              }

              const color = new TinyColor(value);
              if (!color.isValid) {
                return null;
              }

              return normalize(color);
            },
            renderHTML: ({ textBackgroundColor }) => {
              if (!textBackgroundColor) {
                return null;
              }

              return {
                style: `background-color: ${colors[textBackgroundColor as TextBackgroundColor]};`,
              };
            },
          },
        },
      },
    ];
  },

  addCommands() {
    return {
      setTextBackgroundColor:
        (textBackgroundColor) =>
        ({ commands }) => {
          if (!textBackgroundColors.includes(textBackgroundColor)) {
            return false;
          }

          if (textBackgroundColor === defaultValues.textBackgroundColor) {
            return commands.setTextStyle({ textBackgroundColor: null });
          } else {
            return commands.setTextStyle({ textBackgroundColor });
          }
        },
    };
  },
});

const normalize = (color: TinyColor) => {
  const input = color.toRgb();

  return textBackgroundColors.reduce(
    (closest, value) => {
      const target = new TinyColor(hexColors[value]).toRgb();
      const d = Math.hypot(input.r - target.r, input.g - target.g, input.b - target.b);
      return d < closest.d ? { value, d } : closest;
    },
    { value: textBackgroundColors[0], d: Number.MAX_VALUE },
  ).value;
};
