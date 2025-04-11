export type Dragging = {
  entity: Entity;
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

export type Entity = {
  __typename: 'Entity';
  id: string;
  slug: string;
  node?: Post | Folder;
  children?: Entity[];
  order: string;
};

export type Folder = {
  __typename: 'Folder';
  id: string;
  name: string;
  entity?: Entity;
};

export type Post = {
  __typename: 'Post';
  id: string;
  title: string;
  entity?: Entity;
};

export type RootEntity = {
  __typename: 'RootEntity';
  id: null;
  children: Entity[];
};
