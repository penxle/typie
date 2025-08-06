import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { Table } from '../node-views';
import { TEXT_NODE_TYPES, WRAPPING_NODE_NAMES } from './node-commands';

const arrayOrNull = <T>(array: T[] | readonly T[] | null | undefined) => (array?.length ? array : null);
const NODE_TYPES_TO_SELECT_ON_BACKSPACE = [...WRAPPING_NODE_NAMES, Table.name, ...TEXT_NODE_TYPES];

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
          .first(({ commands }) => [
            commands.deleteSelection,
            () => commands.convertNodeToParagraphAtStart(TEXT_NODE_TYPES),
            () => commands.selectNodeBackwardByTypes(NODE_TYPES_TO_SELECT_ON_BACKSPACE),
            commands.joinBackward,
            commands.selectNodeBackward,
          ])
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

      ArrowLeft: ({ editor }) => {
        const { selection, doc } = editor.state;
        const { $from } = selection;

        if ($from.pos > 1) {
          const nodeBefore = doc.resolve($from.pos - 1).nodeBefore;

          if (nodeBefore?.type.name === Table.name) {
            return editor.commands.setTextSelection($from.pos - 5);
          }
        }

        return false;
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
      }),
    ];
  },
});
