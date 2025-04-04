import { Extension } from '@tiptap/core';
import { NodeSelection, Plugin, PluginKey, Selection } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { css, cx } from '$styled-system/css';
import type { Range } from '@tiptap/core';
import type { Node, ResolvedPos } from '@tiptap/pm/model';
import type { Mappable } from '@tiptap/pm/transform';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    blockSelection: {
      setBlockSelection: (range: Range, invisible?: boolean) => ReturnType;
    };
  }
}

export const BlockSelectionExt = Extension.create({
  name: 'block_selection',

  addCommands() {
    return {
      setBlockSelection:
        (range, invisible) =>
        ({ tr, dispatch }) => {
          if (dispatch) {
            const selection = BlockSelection.create(tr.doc, range.from, range.to, invisible);
            tr.setSelection(selection);
          }

          return true;
        },
    };
  },

  addKeyboardShortcuts() {
    return {
      'Mod-a': ({ editor }) => {
        const { $from, $to } = editor.state.selection;

        if ($from.start() === $from.pos && $to.end() === $to.pos) {
          return editor.commands.setBlockSelection({ from: 1, to: editor.state.doc.nodeSize - 2 });
        }

        return editor.commands.setTextSelection({ from: $from.start(), to: $to.end() });
      },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: new PluginKey('block_selection'),
        props: {
          decorations: ({ doc, selection }) => {
            if (!this.editor.isEditable || !(selection instanceof BlockSelection) || selection.invisible) {
              return DecorationSet.empty;
            }

            const body = doc.child(0);
            const { from, to } = selection;
            const decorations: Decoration[] = [];

            body.forEach((node, offset) => {
              if (node.isInline) {
                return;
              }

              const pos = offset + 1;

              const isSelected = from <= pos && to >= pos + node.nodeSize;
              if (!isSelected) {
                return;
              }

              decorations.push(
                Decoration.node(pos, pos + node.nodeSize, {
                  nodeName: 'div',
                  class: cx(
                    'block-selection-decoration',
                    css({
                      '& > *': {
                        position: 'relative',
                        _after: {
                          content: '""',
                          position: 'absolute',
                          inset: '0',
                          borderRadius: '4px',
                          backgroundColor: '[var(--prosemirror-color-selection)/20]',
                          transition: 'common',
                          transitionTimingFunction: 'ease',
                          willChange: 'background-color',
                          pointerEvents: 'none',
                        },
                      },
                    }),
                  ),
                }),
              );
            });

            return DecorationSet.create(doc, decorations);
          },
        },
        appendTransaction: (_, __, newState) => {
          const { selection, tr } = newState;

          if (selection instanceof NodeSelection && selection.node.isBlock) {
            tr.setSelection(BlockSelection.create(newState.doc, selection.from, selection.to));
            return tr;
          }
        },
      }),
    ];
  },
});

export class BlockSelection extends Selection {
  override readonly visible = false;

  constructor(
    from: ResolvedPos,
    to: ResolvedPos,
    public invisible: boolean,
  ) {
    super(from, to);
  }

  override eq(other: Selection) {
    return other instanceof BlockSelection && other.$anchor === this.$anchor && other.$head === this.$head;
  }

  override map(doc: Node, mapping: Mappable): Selection {
    const $head = doc.resolve(mapping.map(this.head));

    if (!$head.parent.inlineContent) {
      return Selection.near($head);
    }

    const $anchor = doc.resolve(mapping.map(this.anchor));

    return new BlockSelection($anchor.parent.inlineContent ? $anchor : $head, $head, this.invisible);
  }

  override toJSON() {
    return { type: 'block', anchor: this.anchor, head: this.head };
  }

  static create(doc: Node, from: number, to: number, invisible = false): BlockSelection {
    return new BlockSelection(doc.resolve(from), doc.resolve(to), invisible);
  }
}

try {
  Selection.jsonID('block', BlockSelection);
} catch {
  // ignore
}
