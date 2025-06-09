import { Extension, getMarkType } from '@tiptap/core';
import { NodeSelection } from '@tiptap/pm/state';
import { ReplaceAroundStep, ReplaceStep } from '@tiptap/pm/transform';
import type { MarkType, Node, NodeType } from '@tiptap/pm/model';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    commands: {
      isMarkAllowed: (typeOrName: string | MarkType) => ReturnType;
      isNodeAllowed: (typeOrName: string | NodeType) => ReturnType;
      insertNode: (node: Node) => ReturnType;
    };
  }
}

export const Command = Extension.create({
  name: 'command',

  addCommands() {
    return {
      isMarkAllowed:
        (typeOrName: string | MarkType) =>
        ({ state, can }) => {
          const type = getMarkType(typeOrName, state.schema);
          if (!can().setMark(type)) {
            return false;
          }

          const { doc, selection } = state;
          const { ranges } = selection;

          for (const { $from, $to } of ranges) {
            let can = $from.depth == 0 ? doc.inlineContent && doc.type.allowsMarkType(type) : false;

            doc.nodesBetween($from.pos, $to.pos, (node) => {
              if (can) {
                return false;
              }

              can = node.inlineContent && node.type.allowsMarkType(type);
            });

            if (can) {
              return true;
            }
          }

          return false;
        },

      isNodeAllowed: () => () => {
        return true;
      },

      insertNode:
        (node) =>
        ({ tr, dispatch, state }) => {
          const { selection } = state;
          let { from, to } = selection;
          const { $from } = selection;

          if ($from.parent.isTextblock) {
            if ($from.parent.content.size === 0) {
              from = $from.before($from.depth);
              to = $from.after($from.depth);
            } else if ($from.parentOffset === 0 && ($from.depth > 1 || $from.index($from.depth - 1) > 0)) {
              from = $from.before($from.depth);
            }
          }

          tr.replaceWith(from, to, node);

          const step = tr.steps.at(-1);
          if (step instanceof ReplaceStep || step instanceof ReplaceAroundStep) {
            const map = tr.mapping.maps[tr.steps.length - 1];
            let insertionStart = 0;
            let insertionEnd = 0;

            map.forEach((_, __, newFrom, newTo) => {
              if (insertionStart === 0) {
                insertionStart = newFrom;
                insertionEnd = newTo;
              }
            });

            const positions = [
              { pos: insertionStart, checkAfter: true },
              { pos: insertionEnd - node.nodeSize, checkAfter: true },
              { pos: Math.max(0, insertionEnd - 1), checkBefore: true },
            ];

            for (const { pos, checkAfter, checkBefore } of positions) {
              if (pos < 0) continue;

              try {
                const $pos = tr.doc.resolve(pos);

                if (checkAfter) {
                  const nodeAfter = $pos.nodeAfter;
                  if (nodeAfter && nodeAfter.type === node.type) {
                    tr.setSelection(NodeSelection.create(tr.doc, pos));
                    break;
                  }
                }

                if (checkBefore) {
                  const nodeBefore = $pos.nodeBefore;
                  if (nodeBefore && nodeBefore.type === node.type) {
                    tr.setSelection(NodeSelection.create(tr.doc, pos - nodeBefore.nodeSize));
                    break;
                  }
                }
              } catch {
                continue;
              }
            }
          }

          dispatch?.(tr);

          return true;
        },
    };
  },
});
