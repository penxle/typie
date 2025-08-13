import { Extension } from '@tiptap/core';
import { EditorState, Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet, EditorView } from '@tiptap/pm/view';
import { css } from '@typie/styled-system/css';
import { token } from '@typie/styled-system/tokens';
import { mmToPx } from '../../utils';

const GAP_HEIGHT = 40;

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
        ({ tr, dispatch }) => {
          this.storage.layout = { width, height, margin };

          tr.setMeta(key, true);
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
    const editor = this.editor;

    return [
      new Plugin({
        key,
        state: {
          init(_, state): { decorations: DecorationSet; pages: number } {
            if (!storage.layout) {
              return { decorations: DecorationSet.empty, pages: 0 };
            }
            const decorations = createDecoration(state, storage.layout);
            return {
              decorations: DecorationSet.create(state.doc, decorations),
              pages: decorations.length,
            };
          },
          apply(tr, value, _oldState, newState): { decorations: DecorationSet; pages: number } {
            if (!storage.layout) {
              return { decorations: DecorationSet.empty, pages: 0 };
            }

            if (!editor.view?.dom?.isConnected) {
              return {
                decorations: value.decorations.map(tr.mapping, tr.doc),
                pages: value.pages,
              };
            }

            const forceUpdate = tr.getMeta(key);
            const pageCount = calculatePageCount(storage.layout, editor.view);
            const currentPageCount = getExistingPageCount(editor.view);

            if (forceUpdate || Math.max(pageCount, 1) !== currentPageCount) {
              const newDecorations = createDecoration(newState, storage.layout);
              return {
                decorations: DecorationSet.create(newState.doc, newDecorations),
                pages: newDecorations.length,
              };
            }

            return {
              decorations: value.decorations.map(tr.mapping, tr.doc),
              pages: value.pages,
            };
          },
        },
        props: {
          decorations(state) {
            return this.getState(state)?.decorations;
          },
        },
        view(view) {
          const updateState = () => {
            if (!storage.layout) return;

            view.dispatch(view.state.tr.setMeta(key, false));
          };

          document.fonts.ready.then(() => {
            updateState();
          });

          updateState();

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

const getExistingPageCount = (view: EditorView) => {
  const editorDom = view.dom;
  const paginationElement = editorDom.querySelector('[data-page-break="true"]');
  if (paginationElement) {
    return paginationElement.children.length;
  }
  return 0;
};

const calculatePageCount = (layout: PageLayout, view: EditorView): number => {
  const { height, margin } = layout;
  const PAGE_HEIGHT_PX = mmToPx(height);
  const MARGIN_PX = mmToPx(margin);
  const CONTENT_HEIGHT = PAGE_HEIGHT_PX - MARGIN_PX * 2;

  const editorDom = view.dom;
  if (!editorDom) return 1;

  const bodyElement = editorDom.firstElementChild as HTMLElement | null;
  if (!bodyElement) return 1;

  const paginationElement = editorDom.querySelector('[data-page-break="true"]');

  const currentPageCount = getExistingPageCount(view);

  if (paginationElement) {
    // NOTE: 어째선지 :last-child로 하면 인용구 같은 걸 중간에 넣었을 때 제대로 못 잡음
    const lastElementOfEditor = [...bodyElement.querySelectorAll('[data-node-id]')].reduce(
      (acc, curr) => {
        if (curr.getBoundingClientRect().bottom > acc.getBoundingClientRect().bottom) {
          return curr;
        }
        return acc;
      },
      bodyElement.querySelector('[data-node-id]') as HTMLElement,
    );
    const lastPageBreak = paginationElement.querySelector('.page-break:last-child .breaker');

    if (lastElementOfEditor && lastPageBreak) {
      const lastPageGap =
        lastElementOfEditor.getBoundingClientRect().bottom - (lastPageBreak as HTMLElement).getBoundingClientRect().bottom;

      if (lastPageGap > 0) {
        // NOTE: 콘텐츠가 마지막 페이지 브레이크보다 아래에 있음 - 페이지 추가 필요
        const addPage = Math.ceil(lastPageGap / CONTENT_HEIGHT);
        return currentPageCount + addPage;
      } else {
        // NOTE: 마지막 콘텐츠가 마지막 페이지 브레이크보다 위에 있음
        const minEmptySpace = -(MARGIN_PX * 2 + GAP_HEIGHT);
        const removePageThreshold = minEmptySpace - CONTENT_HEIGHT;

        if (lastPageGap > minEmptySpace) {
          // NOTE: 빈 공간이 최소값 이내면 현재 페이지 수 유지
          return currentPageCount;
        } else if (lastPageGap < removePageThreshold) {
          // NOTE: 빈 공간이 한 페이지 이상이면 페이지 제거
          const pageHeightWithGap = PAGE_HEIGHT_PX + GAP_HEIGHT;
          const pagesToRemove = Math.floor(lastPageGap / pageHeightWithGap);
          return Math.max(1, currentPageCount + pagesToRemove);
        } else {
          // NOTE: 중간 영역 - 현재 페이지 수 유지
          return currentPageCount;
        }
      }
    }
    return 1;
  } else {
    // NOTE: 초기 상태 - scrollHeight 기반 계산
    const editorHeight = editorDom.scrollHeight;
    const pageCount = Math.ceil(editorHeight / CONTENT_HEIGHT);
    return pageCount <= 0 ? 1 : pageCount;
  }
};

function createDecoration(_state: EditorState, pageOptions: PageLayout): Decoration[] {
  const { width, height, margin } = pageOptions;
  const PAGE_WIDTH_PX = mmToPx(width);
  const PAGE_HEIGHT_PX = mmToPx(height);
  const MARGIN_PX = mmToPx(margin);
  const CONTENT_HEIGHT = PAGE_HEIGHT_PX - MARGIN_PX * 2;

  const pageWidget = Decoration.widget(
    1,
    (view) => {
      const pageCount = calculatePageCount(pageOptions, view);

      const container = document.createElement('div');
      container.className = 'page-breaks-container';
      container.dataset.pageBreak = 'true';
      container.contentEditable = 'false';
      container.style.pointerEvents = 'none';
      container.style.position = 'relative';

      for (let i = 0; i < pageCount; i++) {
        const isFirstPage = i === 0;
        const pageBreak = document.createElement('div');
        pageBreak.className = 'page-break';

        const page = document.createElement('div');
        page.className = 'page';
        page.style.cssText = `
          position: relative;
          float: left;
          clear: both;
          margin-top: ${isFirstPage ? `calc(${CONTENT_HEIGHT}px)` : `${CONTENT_HEIGHT}px`};
        `;

        const pageBackground = document.createElement('div');
        pageBackground.className = 'page-background';
        pageBackground.style.cssText = `
          position: absolute;
          top: ${i * (PAGE_HEIGHT_PX + GAP_HEIGHT) - MARGIN_PX}px;
          left: -${MARGIN_PX}px;
          z-index: -1;
          width: ${PAGE_WIDTH_PX}px;
          height: ${PAGE_HEIGHT_PX}px;
          background-color: ${token('colors.surface.default')};
          box-shadow: ${token('shadows.medium')};
        `;

        const breaker = document.createElement('div');
        breaker.className = 'breaker';
        breaker.style.cssText = `
          width: ${PAGE_WIDTH_PX}px;
          margin-left: calc(50% - ${PAGE_WIDTH_PX / 2}px);
          margin-right: calc(50% - ${PAGE_WIDTH_PX / 2}px);
          position: relative;
          float: left;
          clear: both;
          left: 0px;
          right: 0px;
          z-index: 2;
        `;

        const pageFooter = document.createElement('div');
        pageFooter.className = 'page-footer';
        pageFooter.style.cssText = `
          height: ${MARGIN_PX}px;
        `;

        const paginationGap = document.createElement('div');
        paginationGap.className = 'pagination-gap';
        paginationGap.style.cssText = `
          height: ${GAP_HEIGHT}px;
          position: relative;
          width: 100%;
        `;

        const pageNumber = document.createElement('div');
        pageNumber.textContent = `페이지 ${i + 1} / ${pageCount}`;
        pageNumber.className = css({
          position: 'absolute',
          top: '12px',
          right: '12px',
          transform: 'translateY(-50%)',
          fontSize: '14px',
          color: 'text.faint',
          userSelect: 'none',
        });
        paginationGap.append(pageNumber);

        const pageHeader = document.createElement('div');
        pageHeader.className = 'page-header';
        pageHeader.style.cssText = `
          height: ${MARGIN_PX}px;
        `;

        breaker.append(pageFooter);
        breaker.append(paginationGap);
        breaker.append(pageHeader);

        pageBreak.append(pageBackground);
        pageBreak.append(page);
        pageBreak.append(breaker);

        container.append(pageBreak);
      }

      return container;
    },
    { side: -1 },
  );

  return [pageWidget];
}
