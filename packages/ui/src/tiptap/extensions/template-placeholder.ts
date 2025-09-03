import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet } from '@tiptap/pm/view';
import { mount, unmount } from 'svelte';
import { isBodyEmpty } from '../lib';
import TemplatePlaceholderWidget from './TemplatePlaceholderWidget.svelte';
import type { EditorState } from '@tiptap/pm/state';

export const TemplatePlaceholder = Extension.create({
  name: 'template_placeholder',

  addProseMirrorPlugins() {
    const key = new PluginKey('templatePlaceholder');
    const componentInstances = new Map<HTMLElement, ReturnType<typeof mount>>();

    const createDecorations = (state: EditorState) => {
      if (!this.editor.isEditable || !isBodyEmpty(state)) {
        return DecorationSet.empty;
      }

      const firstParagraph = state.doc.child(0)?.child(0);
      if (!firstParagraph || firstParagraph.type.name !== 'paragraph') {
        return DecorationSet.empty;
      }

      const decoration = Decoration.widget(
        2,
        () => {
          const container = document.createElement('div');
          container.style.position = 'absolute';
          container.style.top = '0';
          container.style.left = '0';
          container.style.width = '100%';

          const component = mount(TemplatePlaceholderWidget, {
            target: container,
            props: {
              editor: this.editor,
            },
          });

          componentInstances.set(container, component);

          return container;
        },
        {
          side: -1,
        },
      );

      return DecorationSet.create(state.doc, [decoration]);
    };

    return [
      new Plugin({
        key,
        state: {
          init: (_, state) => {
            return createDecorations(state);
          },
          apply: (_, decorations, oldState, newState) => {
            const oldEmpty = isBodyEmpty(oldState);
            const newEmpty = isBodyEmpty(newState);

            if (oldEmpty !== newEmpty) {
              componentInstances.forEach((component) => {
                unmount(component);
              });
              componentInstances.clear();

              return createDecorations(newState);
            }

            return decorations;
          },
        },
        props: {
          decorations(state) {
            return this.getState(state);
          },
        },
        destroy() {
          componentInstances.forEach((component) => {
            unmount(component);
          });
          componentInstances.clear();
        },
      }),
    ];
  },
});
