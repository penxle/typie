export type Dragging = {
  item: Item;
  elem: HTMLElement;
  ghostEl: HTMLElement;
  event: PointerEvent;
  pointerId: number;
  moved: boolean;
};

export type DropTarget = {
  elem: HTMLElement | null;
  list: HTMLElement;
  parentId: string | null;
  indicatorPosition: number | null;
};

export type Item = {
  id: string | null;
  type: 'folder' | 'page';
  title: string | null;
  children?: Item[];
};
