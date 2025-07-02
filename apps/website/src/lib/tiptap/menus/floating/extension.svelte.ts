import { autoUpdate, computePosition, flip, hide, offset } from '@floating-ui/dom';
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import { css } from '$styled-system/css';
import Left from './Left.svelte';
import Right from './Right.svelte';

type State = { pos: number | null };

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    anchors: { current: Record<string, string | null> };
  }
}

export const pluginKey = new PluginKey<State>('floating_menu');

export const FloatingMenu = Extension.create({
  name: 'floating_menu',

  addProseMirrorPlugins() {
    if (!this.editor.isEditable || window.__webview__) {
      return [];
    }

    let leftDom: HTMLElement | null = null;
    let leftComponent: Record<string, never> | null = null;
    let leftCleanup: (() => void) | null = null;

    let rightDom: HTMLElement | null = null;
    let rightComponent: Record<string, never> | null = null;
    let rightCleanup: (() => void) | null = null;

    const remove = () => {
      leftCleanup?.();
      leftCleanup = null;
      rightCleanup?.();
      rightCleanup = null;

      if (leftComponent) {
        const d = leftDom;
        leftDom = null;

        unmount(leftComponent, { outro: true }).then(() => {
          d?.remove();
        });

        leftComponent = null;
      }

      if (rightComponent) {
        const d = rightDom;
        rightDom = null;

        unmount(rightComponent, { outro: true }).then(() => {
          d?.remove();
        });

        rightComponent = null;
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

              leftDom = document.createElement('div');
              leftComponent = mount(Left, {
                target: leftDom,
                props: {
                  editor: this.editor,
                  pos: state.pos,
                },
              });

              leftDom.className = css({
                position: 'absolute',
                top: '0',
                left: '0',
                width: 'max',
                visibility: 'hidden',
              });

              document.body.append(leftDom);

              rightDom = document.createElement('div');
              rightComponent = mount(Right, {
                target: rightDom,
                props: {
                  editor: this.editor,
                  pos: state.pos,
                },
              });

              rightDom.className = css({
                position: 'absolute',
                top: '0',
                left: '0',
                width: 'max',
                visibility: 'hidden',
              });

              document.body.append(rightDom);

              leftCleanup?.();
              leftCleanup = autoUpdate(nodeDOM, leftDom, async () => {
                if (!leftDom) {
                  return;
                }

                const { x, y, middlewareData } = await computePosition(nodeDOM, leftDom, {
                  placement: 'left-start',
                  middleware: [offset(16), flip({ padding: 16 }), hide({ padding: 16, strategy: 'escaped' })],
                });

                leftDom.style.left = `${x}px`;
                leftDom.style.top = `${y}px`;
                leftDom.style.visibility = middlewareData.hide?.escaped ? 'hidden' : 'visible';
              });

              rightCleanup?.();
              rightCleanup = autoUpdate(nodeDOM, rightDom, async () => {
                if (!rightDom) {
                  return;
                }

                const { x, y, middlewareData } = await computePosition(nodeDOM, rightDom, {
                  placement: 'right-start',
                  middleware: [offset(16), flip({ padding: 16 }), hide({ padding: 16, strategy: 'escaped' })],
                });

                rightDom.style.left = `${x}px`;
                rightDom.style.top = `${y}px`;
                rightDom.style.visibility = middlewareData.hide?.escaped ? 'hidden' : 'visible';
              });
            },
            destroy: () => {
              leftCleanup?.();
              rightCleanup?.();
              if (leftComponent) {
                unmount(leftComponent);
                leftComponent = null;
              }
              if (rightComponent) {
                unmount(rightComponent);
                rightComponent = null;
              }
              leftDom?.remove();
              rightDom?.remove();
            },
          };
        },
        props: {
          handleDOMEvents: {
            mousemove: (view, event) => {
              const left = view.dom.getBoundingClientRect().left;

              const posAtCoords = view.posAtCoords({ left, top: event.clientY });
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
