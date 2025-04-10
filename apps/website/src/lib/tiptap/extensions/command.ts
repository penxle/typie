import { Extension, getMarkType, getNodeType } from '@tiptap/core';
import type { MarkType, NodeType } from '@tiptap/pm/model';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    commands: {
      isMarkAllowed: (typeOrName: string | MarkType) => ReturnType;
      isNodeAllowed: (typeOrName: string | NodeType) => ReturnType;
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

      isNodeAllowed:
        (typeOrName: string | NodeType) =>
        ({ state }) => {
          const { selection } = state;
          const { $anchor, empty } = selection;

          if (!empty) {
            return false;
          }

          if ($anchor.parent.type.name !== 'paragraph' || $anchor.parent.childCount !== 0) {
            return false;
          }

          const type = getNodeType(typeOrName, state.schema);
          if ($anchor.node($anchor.depth - 1).type.contentMatch.matchType(type)) {
            return true;
          }

          return false;
        },
    };
  },
});
