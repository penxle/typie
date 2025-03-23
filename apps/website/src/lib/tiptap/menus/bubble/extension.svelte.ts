import { autoUpdate, computePosition, hide, inline, offset, shift } from '@floating-ui/dom';
import { Extension, posToDOMRect } from '@tiptap/core';
import { NodeSelection, Plugin, PluginKey, TextSelection } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import { center } from '$styled-system/patterns';
import { BlockSelection } from '../../extensions/block-selection';
import Component from './Component.svelte';
import type { VirtualElement } from '@floating-ui/dom';

type State = { pos: number };

export const BubbleMenu = Extension.create({
  name: 'bubble_menu',

  addProseMirrorPlugins() {
    return [
      new Plugin<State>({
        key: new PluginKey('bubble_menu'),
        view: () => {
          const dom = document.createElement('div');
          let component: Record<string, never> | null = null;
          let cleanup: (() => void) | null;

          const remove = () => {
            dom.style.visibility = 'hidden';
            cleanup?.();
          };

          return {
            update: async (view, oldState) => {
              const selectionChanged = !oldState.selection.eq(view.state.selection);
              const docChanged = !oldState.doc.eq(view.state.doc);

              if (!selectionChanged && !docChanged) {
                return;
              }

              const { selection } = view.state;

              if (selection.empty || selection.from === selection.to || selection instanceof BlockSelection || view.composing) {
                remove();
                return;
              }

              const element: VirtualElement = {
                getBoundingClientRect: () => {
                  if (selection instanceof NodeSelection) {
                    const node = view.nodeDOM(selection.from) as HTMLElement;
                    return node.getBoundingClientRect();
                  } else {
                    return posToDOMRect(view, selection.from, selection.to);
                  }
                },
                getClientRects: () => {
                  if (selection instanceof NodeSelection) {
                    const node = view.nodeDOM(selection.from) as HTMLElement;
                    return node.getClientRects();
                  } else if (selection instanceof TextSelection) {
                    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
                    return globalThis.getSelection()!.getRangeAt(0).getClientRects();
                  } else {
                    return [posToDOMRect(view, selection.from, selection.to)];
                  }
                },
                contextElement: view.dom,
              };

              if (!component) {
                component = mount(Component, {
                  target: dom,
                  props: {
                    editor: this.editor,
                  },
                });

                dom.className = center({
                  position: 'absolute',
                  top: '0',
                  left: '0',
                  width: 'max',
                  visibility: 'hidden',
                });

                document.body.append(dom);
              }

              cleanup?.();
              cleanup = autoUpdate(element, dom, async () => {
                const { x, y, middlewareData } = await computePosition(element, dom, {
                  placement: 'top-start',
                  middleware: [
                    offset({ mainAxis: 8, crossAxis: -16 }),
                    inline(),
                    shift({ mainAxis: false, crossAxis: true, altBoundary: true, padding: 8 }),
                    hide({ padding: -8 }),
                  ],
                });

                if (middlewareData.hide) {
                  dom.style.visibility = middlewareData.hide.referenceHidden ? 'hidden' : 'visible';
                }

                dom.style.left = `${x}px`;
                dom.style.top = `${y}px`;
              });

              dom.style.visibility = 'visible';
            },
            destroy: () => {
              remove();

              if (component) {
                unmount(component);
              }

              dom.remove();
            },
          };
        },
      }),
    ];
  },
});
