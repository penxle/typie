import { createDndHandler } from '@typie/ui/utils';
import type { Action } from 'svelte/action';
import type { DragDropContext, DragView } from './drag-context.svelte';

type DragViewOptions = {
  dragDropContext: DragDropContext;
  viewId: string;
};

export const dragView: Action<HTMLElement, DragViewOptions> = (node, options) => {
  const handler = createDndHandler(node, {
    excludeSelectors: ['button', '[role="button"]', '[role="menu"]', 'a[href]', 'input', 'textarea', 'select'],
    onDragStart: () => {
      const dragItem: DragView = {
        type: 'view',
        viewId: options.viewId,
      };
      options.dragDropContext.startDrag(dragItem);
    },
    onDragEnd: () => {
      if (options.dragDropContext.state.isDragging) {
        options.dragDropContext.drop();
      }
    },
    onDragCancel: () => {
      if (options.dragDropContext.state.isDragging) {
        options.dragDropContext.cancelDrag();
      }
    },
  });

  return {
    update(newOptions: DragViewOptions) {
      options = newOptions;
    },
    destroy() {
      handler.destroy();
    },
  };
};
