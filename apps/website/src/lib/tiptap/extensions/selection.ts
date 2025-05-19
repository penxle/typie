import { Extension } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { NodeSelection, Plugin, Selection as ProseMirrorSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { Tip } from '$lib/notification';
import { css } from '$styled-system/css';
import type { Mappable } from '@tiptap/pm/transform';

export const Selection = Extension.create({
  name: 'selection',

  addKeyboardShortcuts() {
    return {
      'Mod-a': ({ editor }) => {
        const { selection } = editor.state;

        if (selection instanceof NodeSelection || selection instanceof MultiNodeSelection) {
          return editor.commands.command(({ state, tr, dispatch }) => {
            const s = MultiNodeSelection.create(state.doc, 1, state.doc.content.size);
            tr.setSelection(s);
            dispatch?.(tr);

            return true;
          });
        }

        Tip.show('editor.shortcut.select-all', '`Mod-A` 키를 한번 더 눌러 본문 전체를 선택할 수 있어요.');

        return editor.commands.command(({ state, tr, dispatch }) => {
          const s = MultiNodeSelection.create(state.doc, selection.$from.before(), selection.$to.after());
          tr.setSelection(s);
          dispatch?.(tr);

          return true;
        });
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          decorations: (state) => {
            const { doc, selection } = state;

            if (!this.editor.isEditable || (!(selection instanceof NodeSelection) && !(selection instanceof MultiNodeSelection))) {
              return DecorationSet.empty;
            }

            const body = doc.child(0);
            const { from, to } = selection;
            const decorations: Decoration[] = [];

            body.descendants((node, offset) => {
              if (!node.isBlock) {
                return false;
              }

              const pos = offset + 1;
              const selected = from <= pos && to >= pos + node.nodeSize;

              if (!selected) {
                return true;
              }

              decorations.push(
                Decoration.node(pos, pos + node.nodeSize, {
                  nodeName: 'div',
                  class: css({
                    '& > *': {
                      position: 'relative',
                      _after: {
                        content: '""',
                        position: 'absolute',
                        inset: '0',
                        borderRadius: '4px',
                        backgroundColor: '[var(--prosemirror-color-selection)/20]',
                        pointerEvents: 'none',
                      },
                    },
                  }),
                }),
              );

              return false;
            });

            return DecorationSet.create(doc, decorations);
          },
        },
      }),
    ];
  },
});

export class MultiNodeSelection extends ProseMirrorSelection {
  override readonly visible = false;

  override eq(other: ProseMirrorSelection) {
    return other instanceof MultiNodeSelection && other.$anchor === this.$anchor && other.$head === this.$head;
  }

  override map(doc: Node, mapping: Mappable) {
    const $head = doc.resolve(mapping.map(this.head));
    const $anchor = doc.resolve(mapping.map(this.anchor));
    return new MultiNodeSelection($anchor, $head);
  }

  override toJSON() {
    return { type: 'multinode', anchor: this.anchor, head: this.head };
  }

  static create(doc: Node, anchor: number, head: number) {
    return new MultiNodeSelection(doc.resolve(anchor), doc.resolve(head));
  }
}

try {
  ProseMirrorSelection.jsonID('multinode', MultiNodeSelection);
} catch {
  // ignore
}
