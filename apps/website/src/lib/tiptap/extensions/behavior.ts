import { Extension } from '@tiptap/core';

const arrayOrNull = <T>(array: T[] | readonly T[] | null | undefined) => (array?.length ? array : null);

export const Behavior = Extension.create({
  name: 'behavior',

  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { selection, storedMarks } = editor.state;
        const { $anchor, empty } = selection;

        const pos = $anchor.before(2);
        const block = $anchor.node(2);

        if (
          empty &&
          $anchor.parent.isTextblock &&
          $anchor.parent.childCount === 0 &&
          $anchor.parentOffset === 0 &&
          !['paragraph', 'bullet_list', 'ordered_list'].includes(block.type.name) &&
          block.childCount === 0
        ) {
          return editor.chain().setNodeSelection(pos).deleteSelection().insertContentAt(pos, { type: 'paragraph' }).run();
        }

        const marks =
          arrayOrNull(storedMarks) || arrayOrNull($anchor.marks()) || arrayOrNull($anchor.parent.firstChild?.firstChild?.marks) || null;

        return editor
          .chain()
          .first(({ commands }) => [commands.deleteSelection, commands.joinBackward, commands.selectNodeBackward])
          .command(({ tr, dispatch }) => {
            tr.setStoredMarks(marks);
            dispatch?.(tr);
            return true;
          })
          .run();
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

      Tab: ({ editor }) => {
        const { selection } = editor.state;
        const { $anchor } = selection;

        if (editor.isActive('list_item') && $anchor.parentOffset === 0) {
          const res = editor.chain().sinkListItem('list_item').run();
          if (res) {
            return true;
          }
        }

        editor
          .chain()
          .command(({ tr }) => {
            tr.insertText('\u0009');
            return true;
          })
          .run();

        return true;
      },

      'Shift-Tab': ({ editor }) => {
        const { doc, selection } = editor.state;
        const { $anchor } = selection;

        if (editor.isActive('list_item') && $anchor.parentOffset === 0) {
          const res = editor.chain().liftListItem('list_item').run();
          if (res) {
            return true;
          }
        }

        if (doc.textBetween($anchor.pos - 1, $anchor.pos) === '\u0009') {
          editor
            .chain()
            .command(({ tr }) => {
              tr.delete($anchor.pos - 1, $anchor.pos);
              return true;
            })
            .run();

          return true;
        }

        return true;
      },
    };
  },
});
