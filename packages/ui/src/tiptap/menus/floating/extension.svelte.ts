import { autoUpdate, computePosition, flip, hide, offset } from '@floating-ui/dom';
import { Extension, posToDOMRect } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { css } from '@typie/styled-system/css';
import { mount, unmount } from 'svelte';
import { TEXT_NODE_TYPES, WRAPPING_NODE_TYPES } from '../../extensions/node-commands';
import Left from './Left.svelte';
import Right from './Right.svelte';
import type { VirtualElement } from '@floating-ui/dom';
import type { EditorView } from '@tiptap/pm/view';

const LIST_NODE_TYPES = ['bullet_list', 'ordered_list'];
const FLOATING_NODE_TYPES = new Set([...WRAPPING_NODE_TYPES, ...TEXT_NODE_TYPES, ...LIST_NODE_TYPES]);

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

            const nodeDOM = view.nodeDOM(pos) as HTMLElement | null;
            if (!nodeDOM) {
              return;
            }

            const body = view.dom.querySelector('.ProseMirror-body');
            if (!body) {
              return;
            }

            remove();

            // NOTE: 이 노드가 현재 selection을 포함하는지 확인
            const node = view.state.doc.nodeAt(pos);
            const { from, to } = view.state.selection;
            const nodeEnd = pos + (node?.nodeSize ?? 0);
            const isSelectionOverlapping = node && from < nodeEnd && to > pos && from !== to && !(from === pos && to === nodeEnd);

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
              zIndex: 'editor',
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
              zIndex: 'editor',
              visibility: 'hidden',
            });

            document.body.append(rightDom);

            leftCleanup?.();
            leftCleanup = autoUpdate(nodeDOM, leftDom, async () => {
              if (!leftDom) {
                return;
              }

              const bodyRect = (body as HTMLElement).getBoundingClientRect();
              const referenceElement: VirtualElement = {
                getBoundingClientRect: () => {
                  let rect: DOMRect;
                  if (isSelectionOverlapping) {
                    rect = posToDOMRect(view, from, to);
                  } else if (node?.type.name === 'paragraph') {
                    rect = posToDOMRect(view, pos + 1, nodeEnd - 1);
                  } else {
                    rect = posToDOMRect(view, pos, nodeEnd);
                  }
                  return {
                    ...rect,
                    left: bodyRect.left,
                  };
                },
                contextElement: nodeDOM,
              };

              const { x, y, middlewareData } = await computePosition(referenceElement, leftDom, {
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

              const bodyRect = (body as HTMLElement).getBoundingClientRect();
              let nodeRect: DOMRect;
              if (node?.type.name === 'paragraph') {
                nodeRect = posToDOMRect(view, pos + 1, nodeEnd - 1);
              } else {
                nodeRect = posToDOMRect(view, pos, nodeEnd);
              }

              const referenceElement: VirtualElement = {
                getBoundingClientRect: () => {
                  return {
                    ...nodeRect,
                    width: 1,
                    left: bodyRect.right - 1,
                    right: bodyRect.right,
                    x: bodyRect.right - 1,
                  };
                },
                contextElement: nodeDOM,
              };

              const { x, y, middlewareData } = await computePosition(referenceElement, rightDom, {
                placement: 'right-start',
                middleware: [offset(16), hide({ padding: 16, strategy: 'escaped' })],
              });

              const viewportWidth = window.innerWidth;
              const spaceToRight = viewportWidth - x;
              const padding = 16;
              const maxWidth = Math.max(spaceToRight - padding, 0);

              rightDom.style.maxWidth = `${maxWidth}px`;
              rightDom.style.left = `${x}px`;
              rightDom.style.top = `${y}px`;
              rightDom.style.visibility = middlewareData.hide?.escaped ? 'hidden' : 'visible';
            });
          };

          (editorView as ViewWithUpdate).__updateFloatingMenu = updateFloatingMenu;

          return {
            update: (view, prevState) => {
              if (view.dragging) {
                return;
              }

              const state = pluginKey.getState(view.state);
              const prev = pluginKey.getState(prevState);

              if (!state || !prev) {
                return;
              }

              // NOTE: 선택 영역이 변경되었거나 문서가 변경된 경우 업데이트
              const selectionChanged = !view.state.selection.eq(prevState.selection);
              if (state.pos !== prev.pos || !view.state.doc.eq(prevState.doc) || selectionChanged) {
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
              const body = view.dom.querySelector('.ProseMirror-body');
              if (!body) {
                return false;
              }

              const left = body.getBoundingClientRect().left;
              const width = body.getBoundingClientRect().width;

              const posAtCoords = view.posAtCoords({ left: left + width / 2, top: event.clientY });
              if (!posAtCoords || posAtCoords.inside <= 0) {
                const updateFn = (view as ViewWithUpdate).__updateFloatingMenu;
                if (updateFn) {
                  updateFn(view, null);
                }
                return false;
              }

              const pos = posAtCoords.inside;

              const resolvedPos = view.state.doc.resolve(pos);

              let newPos: number | null = null;

              const currentNode = view.state.doc.nodeAt(pos);
              if (currentNode) {
                const nodeType = currentNode.type.name;
                if (FLOATING_NODE_TYPES.has(nodeType)) {
                  newPos = pos;
                }
              }

              if (newPos === null) {
                for (let depth = resolvedPos.depth; depth > 2; depth--) {
                  const node = resolvedPos.node(depth);
                  const nodeType = node.type.name;

                  if (FLOATING_NODE_TYPES.has(nodeType)) {
                    newPos = resolvedPos.before(depth);
                    break;
                  }
                }
              }

              if (newPos === null) {
                newPos = resolvedPos.before(2);
              }

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
