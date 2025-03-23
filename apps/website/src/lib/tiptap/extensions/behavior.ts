import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';

export const Behavior = Extension.create({
  name: 'behavior',

  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { doc, selection } = editor.state;
        const { $anchor, empty } = selection;

        const pos = $anchor.before(2);
        const block = $anchor.node(2);

        if (empty && $anchor.parent.isTextblock && $anchor.parent.childCount === 0 && $anchor.parentOffset === 0) {
          if (!['paragraph', 'bullet_list', 'ordered_list'].includes(block.type.name) && block.childCount === 0) {
            return editor.chain().setNodeSelection(pos).deleteSelection().insertContentAt(pos, { type: 'paragraph' }).run();
          }

          const blockBefore = doc.childBefore(pos).node;
          if (block.childCount === 0 && blockBefore?.isTextblock && blockBefore.childCount === 0) {
            return editor
              .chain()
              .setNodeSelection(pos)
              .deleteSelection()
              .setTextSelection(pos - 1)
              .run();
          }
        }

        return false;
      },

      // Enter: ({ editor }) => {
      //   const { selection } = editor.state;
      //   const { $anchor, empty } = selection;

      //   const pos = $anchor.before(2);
      //   const block = $anchor.node(2);

      //   if (
      //     empty &&
      //     $anchor.parent.isTextblock &&
      //     $anchor.parent.childCount === 0 &&
      //     $anchor.parentOffset === 0 &&
      //     block.type.name !== 'paragraph' &&
      //     block.childCount === 0
      //   ) {
      //     return editor.chain().setNodeSelection(pos).deleteSelection().insertContentAt(pos, { type: 'paragraph' }).run();
      //   }

      //   return false;
      // },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleClick: (view, pos) => {
            const { state } = view;
            const { doc } = state;

            const body = doc.child(0);
            const endOfDocument = pos === doc.content.size - 1;
            const lastChildEmptyParagraph = body.lastChild?.type.name === 'paragraph' && body.lastChild?.childCount === 0;

            if (endOfDocument && !lastChildEmptyParagraph) {
              this.editor
                .chain()
                .insertContentAt(pos, { type: 'paragraph' })
                .setTextSelection(pos + 1)
                .run();
            }
          },
        },
      }),
    ];
  },
});
