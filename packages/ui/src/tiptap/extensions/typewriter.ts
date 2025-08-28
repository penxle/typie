import { Extension } from '@tiptap/core';
import { Plugin } from '@tiptap/pm/state';
import { clamp } from '../../utils';

declare module '@tiptap/core' {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Commands<ReturnType> {
    typewriter: {
      scrollIntoViewFixed: (options?: { pos?: number; animate?: boolean; position?: number }) => ReturnType;
    };
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Storage {
    typewriter: { position?: number };
  }
}

export const Typewriter = Extension.create({
  name: 'typewriter',

  addStorage() {
    return {
      animationId: null,
    };
  },

  addCommands() {
    return {
      scrollIntoViewFixed:
        (options = {}) =>
        ({ editor, dispatch }) => {
          const { pos = editor.state.selection.from, animate = false, position = 0.5 } = options;

          if (dispatch) {
            const coords = editor.view.coordsAtPos(pos);
            const container = editor.view.dom.closest('.editor-scroll-container');
            if (!container) return true;

            const containerRect = container.getBoundingClientRect();
            const cursorTop = coords.top;
            const cursorHeight = coords.bottom - coords.top;

            const availableHeight = containerRect.height - cursorHeight;
            const targetOffset = containerRect.top + availableHeight * position;

            const scrollOffset = cursorTop - targetOffset;
            const currentScrollTop = container.scrollTop;
            const targetScrollTop = currentScrollTop + scrollOffset;

            const maxScrollLength = container.scrollHeight - container.clientHeight;
            const clampedScrollTop = clamp(targetScrollTop, 0, maxScrollLength);

            if (animate) {
              const startScrollTop = container.scrollTop;
              const scrollDistance = clampedScrollTop - startScrollTop;
              const duration = 150;
              const startTime = performance.now();

              const animateScroll = (currentTime: number) => {
                const elapsed = currentTime - startTime;
                const progress = Math.min(elapsed / duration, 1);
                const eased = 1 - Math.pow(1 - progress, 3);

                container.scrollTop = startScrollTop + scrollDistance * eased;

                if (progress < 1) {
                  this.storage.animationId = requestAnimationFrame(animateScroll);
                } else {
                  this.storage.animationId = null;
                }
              };

              if (this.storage.animationId) {
                cancelAnimationFrame(this.storage.animationId);
                this.storage.animationId = null;
              }

              this.storage.animationId = requestAnimationFrame(animateScroll);
            } else {
              requestAnimationFrame(() => {
                container.scrollTop = clampedScrollTop;
              });
            }
          }

          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin({
        props: {
          handleScrollToSelection: () => {
            const position = this.editor.storage.typewriter.position;
            if (position === undefined) {
              return false;
            }

            return this.editor.commands.scrollIntoViewFixed({ animate: false, position });
          },

          handleDOMEvents: {
            keydown: (view) => {
              if (this.editor.storage.typewriter.position === undefined) {
                return;
              }

              const container = view.dom.closest('.editor-scroll-container') as HTMLElement;
              if (!container) return false;

              const scrollTop = container.scrollTop;
              requestAnimationFrame(() => {
                container.scrollTop = scrollTop;
              });
            },
          },
        },
      }),
    ];
  },
});
