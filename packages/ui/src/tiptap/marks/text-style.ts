import { Mark } from '@tiptap/core';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    textStyle: {
      isTextStyleAllowed: () => ReturnType;
      setTextStyle: (attributes: Record<string, unknown>) => ReturnType;
    };
  }
}

export const TextStyle = Mark.create({
  name: 'text_style',
  priority: 101,

  parseHTML() {
    return [{ tag: 'span', getAttrs: (node) => node.hasAttribute('style') && null }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['span', HTMLAttributes, 0];
  },

  addCommands() {
    return {
      isTextStyleAllowed:
        () =>
        ({ can }) => {
          return can().isMarkAllowed(this.type);
        },

      setTextStyle:
        (attributes) =>
        ({ can, commands }) => {
          if (!can().isTextStyleAllowed()) {
            return false;
          }

          return commands.setMark(this.type, attributes);
        },
    };
  },
});
