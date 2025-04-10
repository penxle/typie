import { Mark, mergeAttributes } from '@tiptap/core';
import { css } from '$styled-system/css';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    italic: {
      toggleItalic: () => ReturnType;
    };
  }
}

export const Italic = Mark.create({
  name: 'italic',

  parseHTML() {
    return [
      { tag: 'i' },
      { tag: 'em' },
      {
        style: 'font-style',
        consuming: false,
        getAttrs: (value) => value === 'italic' && null,
      },
    ];
  },

  renderHTML({ HTMLAttributes }) {
    return ['em', mergeAttributes(HTMLAttributes, { class: css({ fontStyle: 'italic' }) }), 0];
  },

  addCommands() {
    return {
      toggleItalic:
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
      'Mod-i': () => this.editor.commands.toggleItalic(),
    };
  },
});
