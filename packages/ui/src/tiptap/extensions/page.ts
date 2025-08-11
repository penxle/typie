import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet, EditorView } from '@tiptap/pm/view';
import { css } from '@typie/styled-system/css';
import { mmToPx } from '../../utils';
import type { Node } from '@tiptap/pm/model';

export type PageLayout = {
  width: number;
  height: number;
  margin: number;
};

export type PageStorage = {
  layout?: PageLayout;
};

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    page: {
      setPageLayout: (layout: PageLayout) => ReturnType;
      clearPageLayout: () => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    page: PageStorage;
  }
}

const key = new PluginKey('page');

export const Page = Extension.create<unknown, PageStorage>({
  name: 'page',

  addStorage() {
    return {};
  },

  addCommands() {
    return {
      setPageLayout:
        ({ width, height, margin }) =>
        ({ tr, dispatch, view, state }) => {
          this.storage.layout = { width, height, margin };

          const pages = calculatePages(this.storage.layout, view, state.doc);

          tr.setMeta(key, pages);
          dispatch?.(tr);

          return true;
        },
      clearPageLayout:
        () =>
        ({ tr, dispatch }) => {
          this.storage.layout = undefined;

          dispatch?.(tr);

          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    const { storage } = this;

    return [
      new Plugin({
        key,
        state: {
          init(): { pos: number; height: number }[] {
            return [];
          },
          apply(tr, pages, oldState, newState) {
            const meta = tr.getMeta(key);
            if (meta) {
              return meta;
            }

            // NOTE: 제자리 드래그의 경우 docChanged는 true이지만 실제 문서는 동일함
            if (!oldState.doc.eq(newState.doc)) {
              return [];
            }

            return pages;
          },
        },
        props: {
          decorations(state) {
            const pages = this.getState(state);
            if (!pages?.length || !storage.layout) {
              return DecorationSet.empty;
            }

            const { height, margin } = storage.layout;
            const PAGE_HEIGHT_PX = mmToPx(height);
            const MARGIN_PX = mmToPx(margin);

            const decorations: Decoration[] = [];

            for (const [i, { pos, height }] of pages.entries()) {
              const widget = Decoration.widget(
                pos,
                () => {
                  const element = document.createElement('div');

                  element.className = css({
                    position: 'relative',
                    height: '40px',
                    backgroundColor: 'surface.muted',
                  });
                  element.dataset.pageGap = 'true';

                  element.style.cssText = `margin: ${MARGIN_PX + (PAGE_HEIGHT_PX - MARGIN_PX * 2 - height)}px -${MARGIN_PX}px ${i === pages.length - 1 ? 0 : MARGIN_PX}px -${MARGIN_PX}px`;

                  const label = document.createElement('span');
                  label.textContent = `페이지 ${i + 1} / ${pages.length}`;
                  label.className = css({
                    position: 'absolute',
                    top: '-32px',
                    right: '20px',
                    fontSize: '12px',
                    color: 'text.faint',
                    userSelect: 'none',
                  });
                  element.append(label);

                  const fill = document.createElement('div');
                  fill.className = css({ position: 'absolute', inset: '0' });
                  element.append(fill);

                  return element;
                },
                { side: -1 },
              );

              decorations.push(widget);
            }

            return DecorationSet.create(state.doc, decorations);
          },
        },
        view(view) {
          const updateState = () => {
            if (!storage.layout) {
              return;
            }

            const pages = calculatePages(storage.layout, view, view.state.doc);
            view.dispatch(view.state.tr.setMeta(key, pages));
          };

          updateState();

          document.fonts.ready.then(() => {
            updateState();
          });

          return {
            update(view, prevState) {
              if (!prevState.doc.eq(view.state.doc)) {
                updateState();
              }
            },
          };
        },
      }),
    ];
  },
});

const calculatePages = (layout: PageLayout, view: EditorView, doc: Node) => {
  const { height, margin } = layout;

  const PAGE_HEIGHT_PX = mmToPx(height);
  const MARGIN_PX = mmToPx(margin);

  const body = doc.firstChild;
  if (!body) {
    return [];
  }

  const pages: { pos: number; height: number }[] = [];

  let pageHeight = 0;
  let firstNode = true;

  body.forEach((_, offset) => {
    const pos = offset + 1;

    try {
      const domNode = view.nodeDOM(pos);
      if (!domNode || !(domNode instanceof HTMLElement)) {
        return;
      }

      const rect = domNode.getBoundingClientRect();
      let nodeHeight = rect.height;

      if (!firstNode) {
        nodeHeight += body.attrs.blockGap * 16;
      }

      if (MARGIN_PX + pageHeight + nodeHeight > PAGE_HEIGHT_PX - MARGIN_PX) {
        pages.push({ pos, height: pageHeight });
        pageHeight = nodeHeight;
        firstNode = true;
      } else {
        pageHeight += nodeHeight;
        firstNode = false;
      }
    } catch {
      // pass
    }
  });

  pages.push({ pos: body.nodeSize - 1, height: pageHeight });

  return pages;
};
