import { Mark, mergeAttributes } from '@tiptap/core';
import { css } from '@typie/styled-system/css';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    underline: {
      toggleUnderline: () => ReturnType;
    };
  }
}

export const Underline = Mark.create({
  name: 'underline',

  parseHTML() {
    return [
      { tag: 'u' },
      {
        style: 'text-decoration-line',
        consuming: false,
        getAttrs: (value) => value.includes('underline') && null,
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['u', mergeAttributes(HTMLAttributes, { class: css({ textDecorationLine: 'underline' }) }), 0];
  },

  addCommands() {
    return {
      toggleUnderline:
        () =>
        ({ commands, can }) => {
          if (!can().isMarkAllowed(this.type)) {
            return false;
          }

          return commands.toggleMark(this.type);
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-u': () => this.editor.commands.toggleUnderline(),
    };
  },
});
