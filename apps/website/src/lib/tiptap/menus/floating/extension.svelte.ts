import { autoUpdate, computePosition, flip, hide, offset } from '@floating-ui/dom';
import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { mount, unmount } from 'svelte';
import { css } from '$styled-system/css';
import Left from './Left.svelte';
import Right from './Right.svelte';
import type { EditorView } from '@tiptap/pm/view';

type State = { pos: number | null };

type ViewWithUpdate = {
  __updateFloatingMenu?: (view: EditorView, pos: number | null) => void;
} & EditorView;

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

    // 내부 상태로 관리할 현재 위치
    let currentPos: number | null = null;

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
        view: (editorView) => {
          const updateFloatingMenu = (view: EditorView, pos: number | null) => {
            if (pos === currentPos) {
              return;
            }

            currentPos = pos;

            if (pos === null) {
              remove();
              return;
            }

            const resolvedPos = view.state.doc.resolve(pos);
            if (resolvedPos.depth !== 1) {
              remove();
              return;
            }

            const nodeDOM = view.nodeDOM(pos) as HTMLElement | null;
            if (!nodeDOM) {
              return;
            }

            remove();

            leftDom = document.createElement('div');
            leftComponent = mount(Left, {
              target: leftDom,
              props: {
                editor: this.editor,
                pos,
              },
            });

            leftDom.className = css({
              position: 'absolute',
              top: '0',
              left: '0',
              width: 'max',
              zIndex: '10',
              visibility: 'hidden',
            });

            document.body.append(leftDom);

            rightDom = document.createElement('div');
            rightComponent = mount(Right, {
              target: rightDom,
              props: {
                editor: this.editor,
                pos,
              },
            });

            rightDom.className = css({
              position: 'absolute',
              top: '0',
              left: '0',
              width: 'max',
              zIndex: '10',
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
          };

          (editorView as ViewWithUpdate).__updateFloatingMenu = updateFloatingMenu;

          return {
            update: (view, prevState) => {
              const state = pluginKey.getState(view.state);
              const prev = pluginKey.getState(prevState);

              if (!state || !prev) {
                return;
              }

              if (state.pos !== prev.pos) {
                updateFloatingMenu(view, state.pos);
              }
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
              delete (editorView as ViewWithUpdate).__updateFloatingMenu;
            },
          };
        },
        props: {
          handleDOMEvents: {
            mousemove: (view, event) => {
              const left = view.dom.getBoundingClientRect().left;

              const posAtCoords = view.posAtCoords({ left, top: event.clientY });
              if (!posAtCoords) {
                const updateFn = (view as ViewWithUpdate).__updateFloatingMenu;
                if (updateFn) {
                  updateFn(view, null);
                }
                return false;
              }

              const pos = posAtCoords.inside === -1 ? posAtCoords.pos : posAtCoords.inside;
              const resolvedPos = view.state.doc.resolve(pos);
              const newPos = resolvedPos.before(2) ?? null;

              const updateFn = (view as ViewWithUpdate).__updateFloatingMenu;
              if (updateFn) {
                updateFn(view, newPos);
              }
              return false;
            },

            keydown: (view) => {
              const updateFn = (view as ViewWithUpdate).__updateFloatingMenu;
              if (updateFn) {
                updateFn(view, null);
              }
              return false;
            },
          },
        },
      }),
    ];
  },
});
