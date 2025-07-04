import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';

const arrayOrNull = <T>(array: T[] | readonly T[] | null | undefined) => (array?.length ? array : null);

export const Behavior = Extension.create({
  name: 'behavior',

  addKeyboardShortcuts() {
    return {
      Backspace: ({ editor }) => {
        const { selection, storedMarks } = editor.state;
        const { $anchor } = selection;

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

  addProseMirrorPlugins() {
    return [
      new Plugin({
        appendTransaction: (transactions, oldState, newState) => {
          if (transactions.some((tr) => tr.storedMarksSet)) {
            return null;
          }

          const { tr } = newState;
          if (oldState.storedMarks?.length) {
            tr.ensureMarks(oldState.storedMarks);
            return tr;
          }

          return null;
        },
        props: {
          handleDOMEvents: {
            cut: (view, event) => {
              event.preventDefault();

              const slice = view.state.selection.content();
              const { dom, text } = view.serializeForClipboard(slice);

              event.clipboardData?.clearData();
              event.clipboardData?.setData('text/html', dom.innerHTML);
              event.clipboardData?.setData('text/plain', text);

              const { tr } = view.state;
              tr.deleteSelection();
              view.dispatch(tr);

              return true;
            },

            copy: (view, event) => {
              event.preventDefault();

              const slice = view.state.selection.content();
              const { dom, text } = view.serializeForClipboard(slice);

              event.clipboardData?.clearData();
              event.clipboardData?.setData('text/html', dom.innerHTML);
              event.clipboardData?.setData('text/plain', text);

              return true;
            },
          },
        },
      }),
    ];
  },
});
