import { TinyColor } from '@ctrl/tinycolor';
import { Mark } from '@tiptap/core';
import { values } from '$lib/tiptap/values';

const hexes = Object.fromEntries(values.fontColor.map(({ value, hex }) => [value, hex]));
const fontColors = values.fontColor.map(({ value }) => value);
type FontColor = (typeof fontColors)[number];

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontColor: {
      setFontColor: (fontColor: FontColor) => ReturnType;
      unsetFontColor: () => ReturnType;
    };
  }
}

export const FontColor = Mark.create({
  name: 'font_color',
  priority: 120,

  addAttributes() {
    return {
      value: {
        parseHTML: (element) => {
          return normalize(element.style.color);
        },
        renderHTML: ({ value }) => ({
          style: `color: ${hexes[value]};${value === 'white' ? ' text-shadow: none !important;' : ''}`,
        }),
      },
    };
  },

  parseHTML() {
    return [
      {
        tag: 'span',
        getAttrs: (node) => {
          const color = new TinyColor((node as HTMLElement).style.color);

          if (color.isValid) {
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
      setFontColor:
        (value) =>
        ({ commands }) => {
          if (!fontColors.includes(value)) {
            return false;
          }

          return commands.setMark(this.name, { value });
        },

      unsetFontColor:
        () =>
        ({ commands }) => {
          return commands.unsetMark(this.name);
        },
    };
  },
});

const normalize = (color: string) => {
  const input = new TinyColor(color).toRgb();

  return fontColors.reduce(
    (closest, value) => {
      const target = new TinyColor(hexes[value]).toRgb();
      const d = Math.hypot(input.r - target.r, input.g - target.g, input.b - target.b);
      return d < closest.d ? { value, d } : closest;
    },
    { value: fontColors[0], d: Number.MAX_VALUE },
  ).value;
};
