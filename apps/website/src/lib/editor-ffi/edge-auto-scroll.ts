import { createDragScroll } from '@typie/ui/utils';
import {
  EDGE_AUTO_SCROLL_MAX_SPEED,
  EDGE_AUTO_SCROLL_MIN_SPEED,
  EDGE_AUTO_SCROLL_THRESHOLD_PX,
  EDGE_AUTO_SCROLL_THROTTLE_MS,
} from './constants';
import type { Editor } from './editor.svelte';

type ClientPoint = {
  clientX: number;
  clientY: number;
};

export class EditorEdgeAutoScroll {
  #dragScroll: ReturnType<typeof createDragScroll> | null = null;
  #onScroll: ((clientX: number, clientY: number) => void) | null = null;

  update(editor: Editor, pointer: ClientPoint, onScroll: (clientX: number, clientY: number) => void): void {
    const viewport = editor.scrollViewport;
    if (!viewport) {
      this.stop();
      return;
    }

    this.stop();
    this.#onScroll = onScroll;
    this.#dragScroll = createDragScroll(viewport, {
      axis: 'both',
      initialPointer: pointer,
      scrollZoneSize: EDGE_AUTO_SCROLL_THRESHOLD_PX,
      minScrollSpeed: EDGE_AUTO_SCROLL_MIN_SPEED,
      maxScrollSpeed: EDGE_AUTO_SCROLL_MAX_SPEED,
      onScrollThrottleMs: EDGE_AUTO_SCROLL_THROTTLE_MS,
      onScroll: (clientX, clientY) => this.#onScroll?.(clientX, clientY),
    });
  }

  stop(): void {
    this.#dragScroll?.destroy();
    this.#dragScroll = null;
    this.#onScroll = null;
  }
}
