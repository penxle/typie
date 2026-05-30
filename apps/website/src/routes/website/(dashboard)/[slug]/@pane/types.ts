import type { LocalStore } from '@typie/ui/state';

export type PaneAxis = {
  id: string;
  type: 'axis';
  direction: 'horizontal' | 'vertical';
  children: Member[];
  flexes: number[];
};

type PaneBase = { id: string; type: 'pane' };

export type Pane = (PaneBase & { kind: 'entity'; slug: string }) | (PaneBase & { kind: 'home' });

export type Member = PaneAxis | Pane;

type _PaneInit<T extends Pane> = T extends unknown ? Omit<T, 'id' | 'type'> : never;
export type PaneInit = _PaneInit<Pane>;

export type PanelTab = 'info' | 'note' | 'anchors' | 'spellcheck' | 'ai' | 'timeline' | 'settings' | 'comment';

export type PaneGroupState = {
  root: Member | null;
  focusedPaneId: string | null;
  panelExpandedByPaneId: Record<string, boolean>;
  panelTabByPaneId: Record<string, PanelTab>;
};

export type DragItem = {
  slug: string;
  type: 'document';
};

export type DragPane = {
  type: 'pane';
  paneId: string;
};

export type DropZone = 'center' | 'left' | 'right' | 'top' | 'bottom';

export type PaneSide = 'left' | 'right' | 'top' | 'bottom';

export type PanePlacement = {
  paneId: string;
  side: PaneSide;
};

export type Rect = { left: number; top: number; width: number; height: number };

export type PaneGroup = {
  state: LocalStore<PaneGroupState>;
  readonly panes: Pane[];
  readonly enabled: boolean;
  addPane: (pane: PaneInit, placement: PanePlacement) => boolean;
  movePane: (paneId: string, placement: PanePlacement) => boolean;
  swapPane: (firstPaneId: string, secondPaneId: string) => boolean;
  removePane: (paneId: string) => boolean;
  replacePane: (paneId: string, pane: PaneInit) => boolean;

  findReplaceOpenByPaneId: Record<string, boolean>;

  handleNavigation: (slug: string, siteId?: string) => void;
  switchToSite: (siteId: string, slug?: string) => void;
  focusPane: (paneId: string) => void;

  resizing: boolean;
  activeZone: { paneId: string; dropZone: DropZone } | null;
  draggingPaneId: string | null;
  rootElement: HTMLElement | null;

  paneRects: Map<string, Rect>;
  hitTest: (x: number, y: number) => { paneId: string; dropZone: DropZone } | null;
  updateActiveZone: (x: number, y: number) => void;
  executeDrop: (item: DragItem | DragPane) => boolean;
  cancelDrag: () => void;
};
