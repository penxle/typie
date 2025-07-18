import { Extension } from '@tiptap/core';
import { redo, undo, yCursorPlugin, ySyncPlugin, yUndoPlugin, yUndoPluginKey } from 'y-prosemirror';
import { css } from '$styled-system/css';
import type { EditorView } from '@tiptap/pm/view';
import type * as YAwareness from 'y-protocols/awareness';
import type * as Y from 'yjs';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    collaboration: {
      undo: () => ReturnType;
      redo: () => ReturnType;
    };
  }
}

type CollaborationOptions = {
  doc: Y.Doc;
  awareness?: YAwareness.Awareness;
};

export const Collaboration = Extension.create<CollaborationOptions>({
  name: 'collaboration',
  priority: 1000,

  addCommands() {
    return {
      undo:
        () =>
        ({ state, tr, dispatch }) => {
          tr.setMeta('preventDispatch', true);

          const undoManager = yUndoPluginKey.getState(state)?.undoManager;
          if (!undoManager || undoManager.undoStack.length === 0) {
            return false;
          }

          if (!dispatch) {
            return true;
          }

          return undo(state);
        },
      redo:
        () =>
        ({ state, tr, dispatch }) => {
          tr.setMeta('preventDispatch', true);

          const undoManager = yUndoPluginKey.getState(state)?.undoManager;
          if (!undoManager || undoManager.redoStack.length === 0) {
            return false;
          }

          if (!dispatch) {
            return true;
          }

          return redo(state);
        },
    };
  },

  addProseMirrorPlugins() {
    const fragment = this.options.doc.getXmlFragment('body');

    const yUndoPluginInstance = yUndoPlugin();
    const originalUndoPluginView = yUndoPluginInstance.spec.view;

    yUndoPluginInstance.spec.view = (view: EditorView) => {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const undoManager = yUndoPluginKey.getState(view.state)?.undoManager as any;

      if (undoManager.restore) {
        undoManager.restore();
        // eslint-disable-next-line @typescript-eslint/no-empty-function
        undoManager.restore = () => {};
      }

      const viewRet = originalUndoPluginView ? originalUndoPluginView(view) : undefined;

      return {
        destroy: () => {
          const hasUndoManSelf = undoManager.trackedOrigins.has(undoManager);
          const observers = undoManager._observers;

          undoManager.restore = () => {
            if (hasUndoManSelf) {
              undoManager.trackedOrigins.add(undoManager);
            }

            undoManager.doc.on('afterTransaction', undoManager.afterTransactionHandler);
            undoManager._observers = observers;
          };

          if (viewRet?.destroy) {
            viewRet.destroy();
          }
        },
      };
    };

    type User = { name: string; color: string };

    const cursorBuilder = (user: User) => {
      const cursor = document.createElement('span');
      cursor.className = css({
        position: 'relative',
        marginX: '-1px',
        borderXWidth: '1px',
        borderColor: '[var(--user-color)]',
        pointerEvents: 'none',
        '& + .ProseMirror-separator': {
          display: 'none',
        },
        '& + .ProseMirror-separator + .ProseMirror-trailingBreak': {
          display: 'none',
        },
        _before: {
          content: 'var(--user-name)',
          position: 'absolute',
          top: '0',
          left: '-1px',
          borderTopLeftRadius: '4px',
          borderTopRightRadius: '4px',
          borderBottomRightRadius: '4px',
          paddingX: '6px',
          paddingY: '4px',
          width: 'max',
          fontFamily: 'ui',
          fontSize: '13px',
          fontWeight: 'medium',
          lineHeight: 'none',
          textIndent: '0',
          color: 'text.bright',
          backgroundColor: '[var(--user-color)]',
          translate: 'auto',
          translateY: '-full',
        },
      });
      cursor.style.setProperty('--user-name', `"${user.name}"`);
      cursor.style.setProperty('--user-color', user.color);
      return cursor;
    };

    const selectionBuilder = (user: User) => {
      return {
        style: `--user-color: color-mix(in srgb, ${user.color} 20%, transparent);`,
        class: css({ backgroundColor: '[var(--user-color)]' }),
      };
    };

    const plugins = [ySyncPlugin(fragment), yUndoPluginInstance];
    if (this.options.awareness) {
      plugins.push(yCursorPlugin(this.options.awareness, { cursorBuilder, selectionBuilder }));
    }

    return plugins;
  },

  addKeyboardShortcuts() {
    return {
      'Mod-z': () => this.editor.commands.undo(),
      'Mod-y': () => this.editor.commands.redo(),
      'Shift-Mod-z': () => this.editor.commands.redo(),
    };
  },
});
