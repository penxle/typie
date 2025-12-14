import { Extension } from '@tiptap/core';
import { dropCursor } from '@tiptap/pm/dropcursor';
import type { Plugin } from '@tiptap/pm/state';

export const DropCursor = Extension.create({
  name: 'drop_cursor',

  addProseMirrorPlugins() {
    return [
      dropCursor({
        class: 'ProseMirror-dropcursor',
        color: false,
        width: 4,
      }) as unknown as Plugin,
    ];
  },
});
