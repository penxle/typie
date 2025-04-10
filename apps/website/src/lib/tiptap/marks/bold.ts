import { Mark, mergeAttributes } from '@tiptap/core';
import { css } from '$styled-system/css';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    bold: {
      toggleBold: () => ReturnType;
    };
  }
}

export const Bold = Mark.create({
  name: 'bold',

  parseHTML() {
    return [
      { tag: 'b' },
      { tag: 'strong' },
      {
        style: 'font-weight',
        consuming: false,
        getAttrs: (value) => {
          if (value === 'bold' || value === 'bolder') {
            return null;
          }

          const weight = Number(value);
          if (Number.isNaN(weight)) {
            return false;
          }

          if (weight >= 500) {
            return null;
          }

          return false;
        },
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['b', mergeAttributes(HTMLAttributes, { class: css({ fontWeight: 'bold' }) }), 0];
  },

  addCommands() {
    return {
      toggleBold:
        () =>
        ({ can, commands }) => {
          if (!can().isMarkAllowed(this.type)) {
            return false;
          }

          return commands.toggleMark(this.type);
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-b': () => this.editor.commands.toggleBold(),
    };
  },
});
