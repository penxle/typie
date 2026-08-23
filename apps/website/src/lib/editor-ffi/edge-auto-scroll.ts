import { createDragScroll } from '@typie/ui/utils';
import { EDGE_AUTO_SCROLL_THROTTLE_MS } from './constants';
import type { ScrollViewport } from '@typie/ui/utils';
import type { Editor } from './editor.svelte';

type ClientPoint = {
  clientX: number;
  clientY: number;
};

export class EditorEdgeAutoScroll {
  #dragScroll: ReturnType<typeof createDragScroll> | null = null;
  #viewport: ScrollViewport | null = null;
  #editor: Editor | null = null;
  #onScroll: ((clientX: number, clientY: number) => void) | null = null;

  update(editor: Editor, pointer: ClientPoint, onScroll: (clientX: number, clientY: number) => void): void {
    const viewport = editor.scrollViewport;
    if (!viewport) {
      this.stop();
      return;
    }

    if (this.#dragScroll && this.#viewport === viewport) {
      this.#editor = editor;
      this.#onScroll = onScroll;
      this.#dragScroll.updatePointer(pointer.clientX, pointer.clientY);
      return;
    }

    this.stop();
    this.#viewport = viewport;
    this.#editor = editor;
    this.#onScroll = onScroll;
    this.#dragScroll = createDragScroll(viewport, {
      axis: 'both',
      initialPointer: pointer,
      onScrollThrottleMs: EDGE_AUTO_SCROLL_THROTTLE_MS,
      onScroll: (clientX, clientY) => {
        this.#editor?.notifyViewportScrolled();
        this.#onScroll?.(clientX, clientY);
      },
    });
  }

  stop(): void {
    this.#dragScroll?.destroy();
    this.#dragScroll = null;
    this.#viewport = null;
    this.#editor = null;
    this.#onScroll = null;
  }
}
