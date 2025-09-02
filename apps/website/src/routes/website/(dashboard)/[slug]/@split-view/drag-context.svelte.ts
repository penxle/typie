import { getContext, setContext } from 'svelte';

export type DragItem = {
  slug: string;
  type: 'post' | 'canvas';
};

export type DragView = {
  type: 'view';
  viewId: string;
};

export type DropZone = 'center' | 'left' | 'right' | 'top' | 'bottom';

type DragDropState = {
  isDragging: boolean;
  dragItem: DragItem | DragView | null;
  droppedItem: DragItem | DragView | null;
};

export class DragDropContext {
  state = $state<DragDropState>({
    isDragging: false,
    dragItem: null,
    droppedItem: null,
  });

  startDrag(item: DragItem | DragView) {
    this.state = {
      isDragging: true,
      dragItem: item,
      droppedItem: null,
    };
  }

  cancelDrag() {
    this.state = {
      isDragging: false,
      dragItem: null,
      droppedItem: null,
    };
  }

  drop() {
    this.state = {
      isDragging: false,
      dragItem: null,
      droppedItem: this.state.dragItem,
    };
  }

  endDrag() {
    this.cancelDrag();
  }
}

const DRAG_DROP_CONTEXT_KEY = Symbol('dragDrop');

export const setupDragDropContext = () => {
  const context = new DragDropContext();
  setContext(DRAG_DROP_CONTEXT_KEY, context);
};

export const getDragDropContext = (): DragDropContext => {
  const context = getContext<DragDropContext>(DRAG_DROP_CONTEXT_KEY);
  if (!context) {
    throw new Error('DragDropContext not found');
  }
  return context;
};
