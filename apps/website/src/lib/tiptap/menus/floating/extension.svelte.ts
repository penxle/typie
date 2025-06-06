import { autoUpdate, computePosition, flip, hide, offset } from '@floating-ui/dom';
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import { css } from '$styled-system/css';
import Component from './Component.svelte';

type State = { pos: number | null };

export const pluginKey = new PluginKey<State>('floating_menu');

export const FloatingMenu = Extension.create({
  name: 'floating_menu',

  addProseMirrorPlugins() {
    if (!this.editor.isEditable || window.__webview__) {
      return [];
    }

    let dom: HTMLElement | null = null;
    let component: Record<string, never> | null = null;
    let cleanup: (() => void) | null = null;

    const remove = () => {
      cleanup?.();
      cleanup = null;

      if (component) {
        const d = dom;
        dom = null;

        unmount(component, { outro: true }).then(() => {
          d?.remove();
        });

        component = null;
      }
    };

    return [
      new Plugin<State>({
        key: pluginKey,
        state: {
          init: () => {
            return { pos: null };
          },
          apply: (tr, prev) => {
            const meta = tr.getMeta(pluginKey);
            if (meta) {
              return meta;
            }

            return prev;
          },
        },
        view: () => {
          return {
            update: (view, prevState) => {
              const state = pluginKey.getState(view.state);
              const prev = pluginKey.getState(prevState);

              if (!state || !prev) {
                return;
              }

              if (state.pos === null) {
                remove();
                return;
              }

              const pos = view.state.doc.resolve(state.pos);
              if (pos.depth !== 1) {
                remove();
                return;
              }

              const nodeDOM = view.nodeDOM(state.pos) as HTMLElement | null;
              if (!nodeDOM) {
                return;
              }

              if (state.pos === prev.pos) {
                return;
              }

              remove();
              dom = document.createElement('div');
              component = mount(Component, {
                target: dom,
                props: {
                  editor: this.editor,
                  pos: state.pos,
                },
              });

              dom.className = css({
                position: 'absolute',
                top: '0',
                left: '0',
                width: 'max',
                visibility: 'hidden',
              });

              document.body.append(dom);

              cleanup?.();
              cleanup = autoUpdate(nodeDOM, dom, async () => {
                if (!dom) {
                  return;
                }

                const { x, y, middlewareData } = await computePosition(nodeDOM, dom, {
                  placement: 'left-start',
                  middleware: [offset(16), flip({ padding: 16 }), hide({ padding: 16, strategy: 'escaped' })],
                });

                dom.style.left = `${x}px`;
                dom.style.top = `${y}px`;
                dom.style.visibility = middlewareData.hide?.escaped ? 'hidden' : 'visible';
              });
            },
            destroy: () => {
              cleanup?.();
              if (component) {
                unmount(component);
                component = null;
              }
              dom?.remove();
            },
          };
        },
        props: {
          handleDOMEvents: {
            mousemove: (view, event) => {
              const posAtCoords = view.posAtCoords({ left: event.clientX, top: event.clientY });
              if (!posAtCoords) {
                return;
              }

              const pos = posAtCoords.inside === -1 ? posAtCoords.pos : posAtCoords.inside;
              const pos$ = view.state.doc.resolve(pos);

              const { tr } = view.state;
              tr.setMeta(pluginKey, { pos: pos$.before(2) ?? null });
              view.dispatch(tr);
            },

            keydown: (view) => {
              const { tr } = view.state;
              tr.setMeta(pluginKey, { pos: null });
              view.dispatch(tr);
            },
          },
        },
      }),
    ];
  },
});
