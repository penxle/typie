import { Mark, mergeAttributes } from '@tiptap/core';
import { css } from '$styled-system/css';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    strike: {
      toggleStrike: () => ReturnType;
    };
  }
}

export const Strike = Mark.create({
  name: 'strike',

  parseHTML() {
    return [
      { tag: 's' },
      { tag: 'del' },
      {
        style: 'text-decoration-line',
        consuming: false,
        getAttrs: (value) => value.includes('line-through') && null,
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['s', mergeAttributes(HTMLAttributes, { class: css({ textDecorationLine: 'line-through' }) }), 0];
  },

  addCommands() {
    return {
      toggleStrike:
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
      'Mod-Shift-s': () => this.editor.commands.toggleStrike(),
    };
  },
});
