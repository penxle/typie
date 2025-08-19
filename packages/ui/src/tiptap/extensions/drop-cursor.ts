import { Extension } from '@tiptap/core';
import { dropCursor } from '@tiptap/pm/dropcursor';
import { Slice } from '@tiptap/pm/model';
import { NodeSelection, Plugin, TextSelection } from '@tiptap/pm/state';
import { MultiNodeSelection } from './selection';

export const DropCursor = Extension.create({
  name: 'drop_cursor',

  addProseMirrorPlugins() {
    return [
      dropCursor({
        class: 'ProseMirror-dropcursor',
        color: false,
        width: 4,
      }),
      new Plugin({
        props: {
          handleDrop(view, event, slice: Slice, moved) {
            if (!view.editable) return false;

            const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
            if (!pos) return false;

            // NOTE: pos.inside가 -1인 경우에만 직접 처리하고 나머지는 기본 핸들러가 처리
            if (pos.inside >= 0) {
              return false;
            }

            const tr = view.state.tr;
            const contentToInsert = slice.content;
            let insertPoint = pos.pos === 0 ? 1 : pos.pos;

            const selection = view.state.selection;
            if (moved) {
              const from = selection.from;
              const to = selection.to;

              if (insertPoint > to) {
                insertPoint = insertPoint - (to - from);
              } else if (insertPoint > from) {
                insertPoint = from;
              }

              tr.delete(from, to);
            }

            const beforeInsert = tr.mapping.map(insertPoint);
            tr.insert(beforeInsert, contentToInsert);

            view.dispatch(tr);

            setTimeout(() => {
              const newTr = view.state.tr;
              const newFrom = beforeInsert;
              const newTo = beforeInsert + contentToInsert.size;

              if (selection instanceof NodeSelection) {
                newTr.setSelection(NodeSelection.create(view.state.doc, beforeInsert));
              } else if (selection instanceof MultiNodeSelection) {
                newTr.setSelection(MultiNodeSelection.create(view.state.doc, newFrom, newTo));
              } else if (selection instanceof TextSelection) {
                newTr.setSelection(TextSelection.create(view.state.doc, newFrom, newTo));
              }

              view.dispatch(newTr);
              view.focus();
            });

            return true;
          },
        },
      }),
    ];
  },
});
