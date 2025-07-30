import { Extension } from '@tiptap/core';
import { Plugin, PluginKey } from '@tiptap/pm/state';
import { Decoration, DecorationSet, EditorView } from '@tiptap/pm/view';
import { css } from '$styled-system/css';
import type { Node } from '@tiptap/pm/model';

type PageLayout = {
  width: number;
  height: number;
  margin: number;
};

type PageStorage = {
  layout?: PageLayout;
};

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    page: {
      setPageLayout: (layout: PageLayout) => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    page: PageStorage;
  }
}

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

          dispatch?.(tr);

          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    const key = new PluginKey('page');
    const { storage } = this;

    const el = document.createElement('div');
    el.style.width = '1mm';
    el.style.position = 'absolute';
    el.style.visibility = 'hidden';
    document.body.append(el);

    const MM_TO_PX = el.getBoundingClientRect().width;
    el.remove();

    const calculatePages = (layout: PageLayout, view: EditorView, doc: Node) => {
      const { height, margin } = layout;

      const PAGE_HEIGHT_PX = height * MM_TO_PX;
      const MARGIN_PX = margin * MM_TO_PX;

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

    return [
      new Plugin({
        key,
        state: {
          init(): { pos: number; height: number }[] {
            return [];
          },
          apply(tr, pages) {
            if (tr.docChanged) {
              return [];
            }

            const meta = tr.getMeta(key);
            if (meta) {
              return meta;
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
            const PAGE_HEIGHT_PX = height * MM_TO_PX;
            const MARGIN_PX = margin * MM_TO_PX;

            const decorations: Decoration[] = [];

            for (const [i, { pos, height }] of pages.entries()) {
              const widget = Decoration.widget(pos, () => {
                const elem = document.createElement('div');

                elem.className = css({
                  position: 'relative',
                  height: '40px',
                  backgroundColor: 'surface.muted',
                });

                elem.style.cssText = `margin: ${MARGIN_PX + (PAGE_HEIGHT_PX - MARGIN_PX * 2 - height)}px -${MARGIN_PX}px ${i === pages.length - 1 ? 0 : MARGIN_PX}px -${MARGIN_PX}px`;

                const label = document.createElement('span');
                label.textContent = `Page ${i + 1} / ${height}px (${PAGE_HEIGHT_PX}px)`;
                label.className = css({
                  position: 'absolute',
                  top: '-32px',
                  right: '20px',
                  fontSize: '12px',
                  color: 'text.faint',
                });

                elem.append(label);

                return elem;
              });

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

            const dom = document.querySelector('.editor') as HTMLElement;
            if (dom) {
              dom.style.setProperty('--prosemirror-max-width', `${storage.layout.width * MM_TO_PX}px`);
              dom.style.setProperty('--prosemirror-page-margin', `${storage.layout.margin * MM_TO_PX}px`);
            }
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
