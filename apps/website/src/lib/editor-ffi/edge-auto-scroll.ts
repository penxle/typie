import { elementScrollViewport, handleDragScroll } from '@typie/ui/utils';
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
  #cleanup: (() => void) | null = null;
  #onScroll: ((clientX: number, clientY: number) => void) | null = null;

  update(editor: Editor, pointer: ClientPoint, onScroll: (clientX: number, clientY: number) => void): void {
    const container = editor.scrollContainerEl;
    if (!container) {
      this.stop();
      return;
    }

    this.stop();
    this.#onScroll = onScroll;
    this.#cleanup =
      handleDragScroll(elementScrollViewport(container), true, {
        axis: 'both',
        initialPointer: pointer,
        scrollZoneSize: EDGE_AUTO_SCROLL_THRESHOLD_PX,
        minScrollSpeed: EDGE_AUTO_SCROLL_MIN_SPEED,
        maxScrollSpeed: EDGE_AUTO_SCROLL_MAX_SPEED,
        onScrollThrottleMs: EDGE_AUTO_SCROLL_THROTTLE_MS,
        onScroll: (clientX, clientY) => this.#onScroll?.(clientX, clientY),
      }) ?? null;
  }

  stop(): void {
    this.#cleanup?.();
    this.#cleanup = null;
    this.#onScroll = null;
  }
}
