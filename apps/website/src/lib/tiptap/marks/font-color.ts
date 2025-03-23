import { TinyColor } from '@ctrl/tinycolor';
import { Mark } from '@tiptap/core';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    fontColor: {
      setFontColor: (fontColor: string) => ReturnType;
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
        parseHTML: (element) => new TinyColor(element.style.color).toHexString(),
        renderHTML: ({ value }) => ({
          style: `color: ${value}`,
        }),
      },
    };
  },

  parseHTML() {
    return [{ tag: 'span', getAttrs: (node) => !!(node as HTMLElement).style.color && null }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', HTMLAttributes, 0];
  },

  addCommands() {
    return {
      setFontColor:
        (value) =>
        ({ commands }) => {
          if (!value.startsWith('#')) {
            return false;
          }

          return commands.setMark(this.name, { value: value.toLowerCase() });
        },

      unsetFontColor:
        () =>
        ({ commands }) => {
          return commands.unsetMark(this.name);
        },
    };
  },
});
