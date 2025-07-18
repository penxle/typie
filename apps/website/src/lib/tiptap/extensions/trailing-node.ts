import { Extension } from '@tiptap/core';
import { EditorState, Plugin, PluginKey } from '@tiptap/pm/state';

export const TrailingNode = Extension.create({
  name: 'trailing_node',

  addProseMirrorPlugins() {
    const key = new PluginKey('trailing_node');

    const needsTrailingNode = (state: EditorState) => {
      const body = state.doc.firstChild;
      if (!body || body.type.name !== 'body') {
        return false;
      }

      const lastNode = body.lastChild;
      if (!lastNode) {
        return true;
      }

      if (lastNode.type.name === 'paragraph') {
        return false;
      }

      return true;
    };

    return [
      new Plugin({
        key,
        appendTransaction: (_, __, newState) => {
          if (!needsTrailingNode(newState)) {
            return;
          }

          const pos = newState.doc.content.size - 1;
          return newState.tr.insert(pos, newState.schema.nodes.paragraph.create());
        },
        state: {
          init: (_, state) => needsTrailingNode(state),
          apply: (tr, value, _, newState) => {
            return tr.docChanged ? needsTrailingNode(newState) : value;
          },
        },
      }),
    ];
  },
});
