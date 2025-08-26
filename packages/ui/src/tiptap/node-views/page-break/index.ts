import { TextSelection } from '@tiptap/pm/state';
import { tick } from 'svelte';
import { createNodeView } from '../../lib';
import Component from './Component.svelte';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    pageBreak: {
      setPageBreak: () => ReturnType;
    };
  }
}

export const PageBreak = createNodeView(Component, {
  name: 'page_break',
  group: 'block',
  atom: true,
  selectable: true,

  addKeyboardShortcuts() {
    return {
      'Mod-Enter': () => this.editor.commands.setPageBreak(),
    };
  },

  addCommands() {
    return {
      setPageBreak:
        () =>
        ({ chain, editor }) => {
          const pageLayout = editor.storage?.page?.layout;
          if (!pageLayout) {
            return false;
          }

          const result = chain()
            .insertNode(this.type.create())
            .command(({ tr }) => {
              const { $to } = tr.selection;
              const endPos = $to.pos;
              const nextPos = Math.min(endPos + 1, tr.doc.content.size);
              tr.setSelection(TextSelection.create(tr.doc, nextPos));
              return true;
            })
            .run();

          tick().then(() => {
            editor.commands.scrollIntoViewFixed({ animate: true, position: 0.8 });
          });

          return result;
        },
    };
  },
});
