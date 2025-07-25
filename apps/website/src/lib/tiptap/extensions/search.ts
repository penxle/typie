import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { DecorationSet } from '@tiptap/pm/view';
import type { Transaction } from '@tiptap/pm/state';

export const searchPluginKey = new PluginKey('search');

export const Search = Extension.create({
  name: 'search',

  addProseMirrorPlugins() {
    return [
      new Plugin({
        key: searchPluginKey,
        state: {
          init() {
            return { decorations: DecorationSet.empty };
          },
          apply(tr: Transaction, value: { decorations: DecorationSet }) {
            const meta = tr.getMeta(searchPluginKey);
            if (meta?.decorations) {
              return { decorations: meta.decorations };
            }
            return value;
          },
        },
        props: {
          decorations(state) {
            return searchPluginKey.getState(state)?.decorations;
          },
        },
      }),
    ];
  },
});
