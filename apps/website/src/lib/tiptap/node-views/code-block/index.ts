import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    codeBlock: {
      setCodeBlock: () => ReturnType;
      insertCodeBlockAt: (pos: number) => ReturnType;
    };
  }
}

export const CodeBlock = createNodeView(Component, {
  name: 'code_block',
  group: 'block',
  content: 'text*',
  marks: '',
  code: true,

  parseHTML() {
    return [{ tag: 'pre' }];
  },

  addAttributes() {
    return {
      language: {
        default: 'text',
      },
    };
  },

  addCommands() {
    return {
      setCodeBlock:
        () =>
        ({ can, commands }) => {
          if (!can().isNodeAllowed(this.name)) {
            return false;
          }

          return commands.insertContent({ type: this.name });
        },
      insertCodeBlockAt:
        (pos) =>
        ({ chain }) => {
          return chain().insertContentAt(pos, { type: this.name }).focus().run();
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { $anchor, empty } = editor.state.selection;

        if (!empty || $anchor.parent.type !== this.type || $anchor.parent.textContent.length > 0) {
          return false;
        }

        return true;
      },

      'Mod-a': ({ editor }) => {
        const { $anchor } = editor.state.selection;
        if ($anchor.parent.type !== this.type) {
          return false;
        }

        return editor.commands.setTextSelection({
          from: $anchor.start(),
          to: $anchor.end(),
        });
      },
    };
  },
});
