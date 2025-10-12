import { createDndHandler } from '@typie/ui/utils';
import type { Action } from 'svelte/action';
import type { WidgetType } from './widget-context.svelte';

type DragPaletteWidgetOptions = {
  widgetType: WidgetType;
  onDragStart: (e: PointerEvent) => void;
  onDragMove?: (e: PointerEvent) => void;
  onDragEnd: (e: PointerEvent) => void;
  onDragCancel: () => void;
  isAdded: boolean;
};

export const dragPaletteWidget: Action<HTMLElement, DragPaletteWidgetOptions> = (node, options) => {
  const handler = createDndHandler(node, {
    excludeSelectors: ['button'],
    canStartDrag: () => !options.isAdded,
    onDragStart: (e) => {
      options.onDragStart(e);
    },
    onDragMove: (e) => {
      options.onDragMove?.(e);
    },
    onDragEnd: (e) => {
      options.onDragEnd(e);
    },
    onDragCancel: () => {
      options.onDragCancel();
    },
  });

  return {
    update(newOptions: DragPaletteWidgetOptions) {
      options = newOptions;
    },
    destroy() {
      handler.destroy();
    },
  };
};
