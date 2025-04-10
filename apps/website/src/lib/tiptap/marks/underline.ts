import { Mark, mergeAttributes } from '@tiptap/core';
import { css } from '$styled-system/css';

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
    return [{ tag: 'u' }];
  },

  renderHTML({ HTMLAttributes }) {
    return ['u', mergeAttributes(HTMLAttributes, { class: css({ textDecorationLine: 'underline' }) }), 0];
  },

  addCommands() {
    return {
      toggleUnderline:
        () =>
        ({ commands, can }) => {
          if (!can().isMarkAllowed(this.name)) {
            return false;
          }

          return commands.toggleMark(this.name);
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-u': () => this.editor.commands.toggleUnderline(),
    };
  },
});
