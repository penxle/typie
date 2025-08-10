import { Extension } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { NodeSelection, Plugin, Selection as ProseMirrorSelection } from '@tiptap/pm/state';
import { CellSelection } from '@tiptap/pm/tables';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { css, cx } from '@typie/styled-system/css';
import { Tip } from '../../notification';
import { TEXT_NODE_TYPES, WRAPPING_NODE_TYPES } from './node-commands';
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
        const { from, to } = selection;

        if (selection instanceof CellSelection) {
          let tableDepth = $from.depth;
          while (tableDepth >= 2 && $from.node(tableDepth).type.name !== 'table') {
            tableDepth--;
          }

          if (tableDepth >= 2 && $from.node(tableDepth).type.name === 'table') {
            const tablePos = $from.before(tableDepth);
            return editor.commands.command(({ state, tr, dispatch }) => {
              const s = NodeSelection.create(state.doc, tablePos);
              tr.setSelection(s);
              dispatch?.(tr);
              return true;
            });
          }
        }

        // NOTE: TEXT_NODE_TYPES (CodeBlock, HtmlBlock) 특별 처리
        if (TEXT_NODE_TYPES.includes($from.parent.type.name)) {
          const blockStart = $from.start();
          const blockEnd = $from.end();
          const isEmpty = blockStart === blockEnd;
          const isFullySelected = from === blockStart && to === blockEnd;

          if (isFullySelected || isEmpty) {
            return editor.commands.command(({ state, tr, dispatch }) => {
              const s = NodeSelection.create(state.doc, $from.before($from.depth));
              tr.setSelection(s);
              dispatch?.(tr);
              return true;
            });
          }

          if (from === to || from > blockStart || to < blockEnd) {
            return editor.commands.setTextSelection({ from: blockStart, to: blockEnd });
          }
        }

        let depth = $from.depth;
        while (depth >= 1) {
          const node = $from.node(depth);
          const nodeStart = $from.before(depth);
          const nodeEnd = $from.after(depth);
          const isWrappingNode = WRAPPING_NODE_TYPES.includes(node.type.name);

          if (isWrappingNode) {
            const innerStart = nodeStart + 1;
            const innerEnd = nodeEnd - 1;
            const isInnerFullySelected = from === innerStart && to === innerEnd;

            if (isInnerFullySelected) {
              return editor.commands.command(({ state, tr, dispatch }) => {
                const s = NodeSelection.create(state.doc, nodeStart);
                tr.setSelection(s);
                dispatch?.(tr);
                return true;
              });
            }

            if (selection instanceof NodeSelection && !isInnerFullySelected) {
              return editor.commands.command(({ state, tr, dispatch }) => {
                const s = MultiNodeSelection.create(state.doc, innerStart, innerEnd);
                tr.setSelection(s);
                dispatch?.(tr);
                return true;
              });
            }
          }

          const needsExpansion = from > nodeStart || to < nodeEnd;
          if (needsExpansion) {
            if (depth === 1) {
              return editor.commands.command(({ state, tr, dispatch }) => {
                const s = MultiNodeSelection.create(state.doc, 1, state.doc.content.size);
                tr.setSelection(s);
                dispatch?.(tr);
                return true;
              });
            }

            return editor.commands.command(({ state, tr, dispatch }) => {
              const s = NodeSelection.create(state.doc, nodeStart);
              tr.setSelection(s);
              dispatch?.(tr);

              Tip.show('editor.shortcut.expand-selection', '`Mod-A`를 계속 누르면 선택 영역이 확장되어요.');

              return true;
            });
          }

          depth--;
        }

        return editor.commands.command(({ state, tr, dispatch }) => {
          const s = MultiNodeSelection.create(state.doc, 1, state.doc.content.size);
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
            const { from, to } = selection;
            const decorations: Decoration[] = [];

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
