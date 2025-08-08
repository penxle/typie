import { Extension, isiOS } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { NodeSelection, Plugin, Selection as ProseMirrorSelection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { css, cx } from '@typie/styled-system/css';
import { Tip } from '$lib/notification';
import { TEXT_NODE_TYPES } from './node-commands';
import type { Mappable } from '@tiptap/pm/transform';

export const Selection = Extension.create({
  name: 'selection',

  addKeyboardShortcuts() {
    return {
      'Mod-a': ({ editor }) => {
        if (!this.editor.isEditable) {
          return false;
        }

        const { selection } = editor.state;
        const { $from } = selection;

        if (TEXT_NODE_TYPES.includes($from.parent.type.name)) {
          const parentStart = $from.start();
          const parentEnd = $from.end();

          if (selection.from !== parentStart || selection.to !== parentEnd) {
            // NOTE: 모든 텍스트가 선택되어 있지 않으면 전체 텍스트 내용 선택
            editor.commands.setTextSelection({
              from: parentStart,
              to: parentEnd,
            });
            return true;
          }
        }

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

            const body = doc.child(0);
            const { from, to, empty } = selection;
            const decorations: Decoration[] = [];

            if (!empty && (isiOS() || window.__webview__?.platform === 'ios')) {
              decorations.push(Decoration.inline(from, to, { class: cx('selected-text') }));
            }

            if (this.editor.isEditable && (selection instanceof NodeSelection || selection instanceof MultiNodeSelection)) {
              const startPos = Math.max(1, from - 1);
              const endPos = Math.min(body.content.size + 1, to + 1);

              body.nodesBetween(startPos - 1, endPos - 1, (node, offset) => {
                if (!node.isBlock) {
                  return true;
                }

                if (node.type.name === 'table_row' || node.type.name === 'table_cell') {
                  return true;
                }

                const pos = offset + 1;
                const selected = from <= pos && to >= pos + node.nodeSize;

                if (!selected) {
                  return true;
                }

                decorations.push(
                  Decoration.node(pos, pos + node.nodeSize, {
                    nodeName: 'div',
                    class: cx(
                      'selected-node',
                      css({
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
                    ),
                  }),
                );

                return true;
              });
            }

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
