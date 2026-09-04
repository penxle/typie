import { pointerCapture } from '@typie/ui/actions';
import { pushEscapeHandler } from '@typie/ui/utils';
import type { PointerCaptureCancelReason } from '@typie/ui/actions';
import type { Action } from 'svelte/action';
import type { PaneGroup } from '../[slug]/@pane/context.svelte';

const TOUCH_DRAG_HOLD_MS = 350;
const DRAG_MOVE_THRESHOLD_PX = 10;
export const UNPIN_HOLD_MS = 500;

export type EntityRowDragItem = {
  id: string;
  type: 'document' | 'folder';
  slug: string;
  name: string;
  icon?: string;
};

export type EntityRowDrop =
  { kind: 'reorder'; lowerOrder: string | null; upperOrder: string | null } | { kind: 'pin' } | { kind: 'outside' };

export type EntityRowDropResult = Exclude<EntityRowDrop, { kind: 'outside' }> | { kind: 'unpin' };

export type EntityRowDragGhost = EntityRowDragItem & {
  x: number;
  y: number;
  width: number;
  cue?: string;
};

type PointerSession = {
  pointerId: number;
  pointerType: 'mouse' | 'pen' | 'touch';
  startX: number;
  startY: number;
  lastY: number;
  element: HTMLElement;
  scrollSurface: HTMLElement;
  item: EntityRowDragItem;
  active: boolean;
  touchHoldCanceled: boolean;
  holdTimeout?: ReturnType<typeof setTimeout>;
  outsideHoldTimeout?: ReturnType<typeof setTimeout>;
  outsideHoldArmed: boolean;
};

type ClickSuppression = {
  pointerId: number;
  element: HTMLElement;
  expiryTimeout?: ReturnType<typeof setTimeout>;
};

type FinishActiveOptions = {
  cancelPane: boolean;
  suppressClick: boolean;
};

type CancelOptions = {
  suppressClick?: boolean;
};

type EntityRowDragOptions = {
  paneGroup: PaneGroup;
  onDropSuccess?: () => void;
  resolveDrop?: (x: number, y: number, item: EntityRowDragItem) => EntityRowDrop | null;
  onDrop?: (drop: EntityRowDropResult, item: EntityRowDragItem) => void;
  holdOutside?: { ms: number; cue: string };
};

const sameDrop = (a: EntityRowDrop | null, b: EntityRowDrop | null) => {
  if (a === b) return true;
  if (!a || !b || a.kind !== b.kind) return false;
  if (a.kind === 'reorder' && b.kind === 'reorder') return a.lowerOrder === b.lowerOrder && a.upperOrder === b.upperOrder;
  return true;
};

export class EntityRowDragController {
  #paneGroup: PaneGroup;
  #onDropSuccess?: () => void;
  #resolveDrop?: EntityRowDragOptions['resolveDrop'];
  #onDrop?: EntityRowDragOptions['onDrop'];
  #holdOutside?: EntityRowDragOptions['holdOutside'];
  #session = $state.raw<PointerSession | null>(null);
  #active = $state(false);
  #clickSuppression: ClickSuppression | null = null;
  #cancelCapture: (() => void) | null = null;
  #removeEscapeHandler: (() => void) | null = null;
  #suppressProgrammaticCancelClick = false;

  ghost = $state<EntityRowDragGhost | null>(null);
  drop = $state<EntityRowDrop | null>(null);

  drag: Action<HTMLElement, EntityRowDragItem | null> = (element, initialItem) => {
    let item = initialItem;
    const capture = pointerCapture<PointerSession>(element, {
      start: (event) => {
        if (!item) return null;
        const session = this.#start(event, element, item);
        if (session) this.#cancelCapture = capture.cancel;
        return session;
      },
      move: (session, event) => this.#move(session, event),
      end: (session, event) => this.#end(session, event),
      cancel: (session, reason) => this.#cancelSession(session, reason),
    });

    return {
      update(nextItem) {
        item = nextItem;
      },
      destroy: capture.destroy,
    };
  };

  constructor({ paneGroup, onDropSuccess, resolveDrop, onDrop, holdOutside }: EntityRowDragOptions) {
    this.#paneGroup = paneGroup;
    this.#onDropSuccess = onDropSuccess;
    this.#resolveDrop = resolveDrop;
    this.#onDrop = onDrop;
    this.#holdOutside = holdOutside;
  }

  #clearClickSuppression(pointerId?: number): void {
    const suppression = this.#clickSuppression;
    if (!suppression || (pointerId !== undefined && suppression.pointerId !== pointerId)) return;
    if (suppression.expiryTimeout) clearTimeout(suppression.expiryTimeout);
    this.#clickSuppression = null;
  }

  #armClickSuppression(session: PointerSession): void {
    this.#clearClickSuppression();
    this.#clickSuppression = { pointerId: session.pointerId, element: session.element };
  }

  #expireClickSuppressionAfterPointerUp(pointerId: number): void {
    const suppression = this.#clickSuppression;
    if (!suppression || suppression.pointerId !== pointerId) return;
    suppression.expiryTimeout = setTimeout(() => {
      if (this.#clickSuppression === suppression) this.#clickSuppression = null;
    }, 0);
  }

  #ghostFor(session: PointerSession, x: number, y: number): EntityRowDragGhost {
    return {
      ...session.item,
      x,
      y,
      width: session.element.offsetWidth,
      cue: session.outsideHoldArmed ? this.#holdOutside?.cue : undefined,
    };
  }

  #setDrop(drop: EntityRowDrop | null): void {
    if (sameDrop(this.drop, drop)) return;
    this.drop = drop;
  }

  #clearOutsideHold(session: PointerSession): void {
    if (session.outsideHoldTimeout) clearTimeout(session.outsideHoldTimeout);
    session.outsideHoldTimeout = undefined;
    if (!session.outsideHoldArmed) return;
    session.outsideHoldArmed = false;
    if (this.ghost) this.ghost = { ...this.ghost, cue: undefined };
  }

  #armOutsideHold(session: PointerSession): void {
    const holdOutside = this.#holdOutside;
    if (!holdOutside || session.outsideHoldArmed || session.outsideHoldTimeout) return;
    session.outsideHoldTimeout = setTimeout(() => {
      session.outsideHoldTimeout = undefined;
      if (this.#session !== session || !session.active) return;
      session.outsideHoldArmed = true;
      if (this.ghost) this.ghost = { ...this.ghost, cue: holdOutside.cue };
    }, holdOutside.ms);
  }

  #resolve(session: PointerSession, x: number, y: number): void {
    const resolved = this.#resolveDrop?.(x, y, session.item) ?? null;

    if (resolved && resolved.kind !== 'outside') {
      this.#clearOutsideHold(session);
      if (this.#paneGroup.activeZone) this.#paneGroup.cancelDrag();
      this.#setDrop(resolved);
      return;
    }

    if (session.item.type === 'document') {
      this.#paneGroup.updateActiveZone(x, y);
      if (this.#paneGroup.activeZone) {
        this.#clearOutsideHold(session);
        this.#setDrop(null);
        return;
      }
    } else if (this.#paneGroup.activeZone) {
      this.#paneGroup.cancelDrag();
    }

    if (resolved?.kind === 'outside') {
      this.#armOutsideHold(session);
    } else {
      this.#clearOutsideHold(session);
    }
    this.#setDrop(null);
  }

  #activate(session: PointerSession, x: number, y: number): void {
    if (this.#session !== session || session.active || session.touchHoldCanceled) return;
    if (session.holdTimeout) clearTimeout(session.holdTimeout);
    session.holdTimeout = undefined;
    session.active = true;
    this.#active = true;
    this.updatePointer(x, y);
  }

  #finish(session: PointerSession, { cancelPane, suppressClick }: FinishActiveOptions): void {
    if (this.#session !== session) return;
    this.#clearOutsideHold(session);
    this.#session = null;
    this.#active = false;
    this.#cancelCapture = null;
    this.#removeEscapeHandler?.();
    this.#removeEscapeHandler = null;
    this.ghost = null;
    this.drop = null;

    if (session.holdTimeout) clearTimeout(session.holdTimeout);
    if (cancelPane) this.#paneGroup.cancelDrag();
    if (suppressClick) this.#armClickSuppression(session);
  }

  #start(event: PointerEvent, element: HTMLElement, item: EntityRowDragItem): PointerSession | null {
    if (this.#session || !event.isPrimary || event.button !== 0) return null;
    if (event.pointerType !== 'mouse' && event.pointerType !== 'pen' && event.pointerType !== 'touch') return null;

    const interactiveTarget =
      event.target instanceof Element
        ? event.target.closest<HTMLElement>('button, [role="button"], [role="menu"], a[href], input, textarea, select')
        : null;
    if (interactiveTarget && interactiveTarget !== element) return null;

    const scrollSurface = element.closest<HTMLElement>('[data-entity-row-drag-scroll-surface], [role="tree"]');
    if (!scrollSurface) return null;
    this.#clearClickSuppression();

    const session: PointerSession = {
      pointerId: event.pointerId,
      pointerType: event.pointerType,
      startX: event.clientX,
      startY: event.clientY,
      lastY: event.clientY,
      element,
      scrollSurface,
      item,
      active: false,
      touchHoldCanceled: false,
      outsideHoldArmed: false,
    };
    this.#session = session;
    this.#removeEscapeHandler = pushEscapeHandler(() => {
      this.cancel({ suppressClick: true });
      return true;
    });

    if (session.pointerType === 'touch') {
      session.holdTimeout = setTimeout(() => {
        if (this.#session !== session) return;
        this.#activate(session, session.startX, session.startY);
      }, TOUCH_DRAG_HOLD_MS);
    }

    return session;
  }

  #move(session: PointerSession, event: PointerEvent): void {
    if (!session.active) {
      const distance = Math.abs(event.clientX - session.startX) + Math.abs(event.clientY - session.startY);

      if (session.pointerType === 'touch') {
        const deltaY = event.clientY - session.lastY;
        session.lastY = event.clientY;

        if (!session.touchHoldCanceled && distance > DRAG_MOVE_THRESHOLD_PX) {
          session.touchHoldCanceled = true;
          if (session.holdTimeout) clearTimeout(session.holdTimeout);
          session.holdTimeout = undefined;
        }

        if (session.touchHoldCanceled) {
          if (event.cancelable) event.preventDefault();
          session.scrollSurface.scrollTop -= deltaY;
        }
        return;
      }

      if (distance <= DRAG_MOVE_THRESHOLD_PX) return;

      this.#activate(session, event.clientX, event.clientY);
      return;
    }

    if (event.cancelable) event.preventDefault();
    this.updatePointer(event.clientX, event.clientY);
  }

  #end(session: PointerSession, event: PointerEvent): void {
    if (!session.active) {
      if (session.touchHoldCanceled) this.#armClickSuppression(session);
      this.#finish(session, { cancelPane: false, suppressClick: false });
      this.#expireClickSuppressionAfterPointerUp(event.pointerId);
      return;
    }

    this.#resolve(session, event.clientX, event.clientY);
    const item = session.item;
    const drop = this.drop;
    let result: EntityRowDropResult | null = null;
    let paneSucceeded = false;

    if (drop && drop.kind !== 'outside') {
      result = drop;
    } else if (this.#paneGroup.activeZone !== null) {
      paneSucceeded = this.#paneGroup.executeDrop({ slug: item.slug, type: 'document' });
    } else if (session.outsideHoldArmed) {
      result = { kind: 'unpin' };
    }

    this.#finish(session, { cancelPane: !paneSucceeded, suppressClick: true });
    this.#expireClickSuppressionAfterPointerUp(event.pointerId);
    if (paneSucceeded) this.#onDropSuccess?.();
    if (result) this.#onDrop?.(result, item);
  }

  #cancelSession(session: PointerSession, reason: PointerCaptureCancelReason): void {
    const suppressClick =
      reason === 'lostpointercapture' ? session.active : reason === 'programmatic' ? this.#suppressProgrammaticCancelClick : false;
    this.#suppressProgrammaticCancelClick = false;
    this.#finish(session, { cancelPane: session.active, suppressClick });
    if (!suppressClick) this.#clearClickSuppression(session.pointerId);
  }

  get hasPointerSession(): boolean {
    return this.#session !== null;
  }

  get active(): boolean {
    return this.#active;
  }

  updatePointer(x: number, y: number): void {
    const session = this.#session;
    if (!session?.active) return;
    this.ghost = this.#ghostFor(session, x, y);
    this.#resolve(session, x, y);
  }

  contextMenu(event: MouseEvent): void {
    const session = this.#session;
    if (!session || session.pointerType !== 'touch' || session.touchHoldCanceled) return;
    event.preventDefault();
    event.stopPropagation();
  }

  consumeClick(event: MouseEvent): boolean {
    const suppression = this.#clickSuppression;
    if (!suppression) return false;
    this.#clearClickSuppression();
    if (event.currentTarget !== suppression.element) return false;
    event.preventDefault();
    event.stopImmediatePropagation();
    return true;
  }

  cancel({ suppressClick = false }: CancelOptions = {}): boolean {
    const session = this.#session;
    if (!session) return false;
    this.#suppressProgrammaticCancelClick = suppressClick;
    this.#cancelCapture?.();
    if (!suppressClick) this.#clearClickSuppression();
    return true;
  }

  destroy(): void {
    this.cancel();
    this.#clearClickSuppression();
  }
}
