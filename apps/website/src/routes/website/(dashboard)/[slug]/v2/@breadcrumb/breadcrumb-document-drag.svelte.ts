import { pointerCapture } from '@typie/ui/actions';
import type { PointerCaptureCancelReason } from '@typie/ui/actions';
import type { Action } from 'svelte/action';
import type { PaneGroup } from '../../@pane/context.svelte';

const TOUCH_DRAG_HOLD_MS = 350;
const DRAG_MOVE_THRESHOLD_PX = 10;

type DocumentDragItem = {
  slug: string;
  name: string;
  icon?: string;
};

type PointerSession = {
  pointerId: number;
  pointerType: 'mouse' | 'pen' | 'touch';
  startX: number;
  startY: number;
  lastY: number;
  element: HTMLElement;
  scrollSurface: HTMLElement;
  item: DocumentDragItem;
  active: boolean;
  touchHoldCanceled: boolean;
  holdTimeout?: ReturnType<typeof setTimeout>;
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

export type BreadcrumbDocumentDragGhost = DocumentDragItem & {
  x: number;
  y: number;
  width: number;
};

type BreadcrumbDocumentDragOptions = {
  paneGroup: PaneGroup;
  onDropSuccess: () => void;
};

export class BreadcrumbDocumentDragController {
  #paneGroup: PaneGroup;
  #onDropSuccess: () => void;
  #session = $state.raw<PointerSession | null>(null);
  #clickSuppression: ClickSuppression | null = null;
  #cancelCapture: (() => void) | null = null;
  #suppressProgrammaticCancelClick = false;

  ghost = $state<BreadcrumbDocumentDragGhost | null>(null);

  drag: Action<HTMLElement, DocumentDragItem | null> = (element, initialItem) => {
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

  constructor({ paneGroup, onDropSuccess }: BreadcrumbDocumentDragOptions) {
    this.#paneGroup = paneGroup;
    this.#onDropSuccess = onDropSuccess;
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

  #activate(session: PointerSession, x: number, y: number): void {
    if (this.#session !== session || session.active || session.touchHoldCanceled) return;
    if (session.holdTimeout) clearTimeout(session.holdTimeout);
    session.holdTimeout = undefined;
    session.active = true;

    this.ghost = { ...session.item, x, y, width: session.element.offsetWidth };
    this.#paneGroup.updateActiveZone(x, y);
  }

  #finish(session: PointerSession, { cancelPane, suppressClick }: FinishActiveOptions): void {
    if (this.#session !== session) return;
    this.#session = null;
    this.#cancelCapture = null;
    this.ghost = null;

    if (session.holdTimeout) clearTimeout(session.holdTimeout);
    if (cancelPane) this.#paneGroup.cancelDrag();
    if (suppressClick) this.#armClickSuppression(session);
  }

  #start(event: PointerEvent, element: HTMLElement, item: DocumentDragItem): PointerSession | null {
    if (this.#session || !event.isPrimary || event.button !== 0) return null;
    if (event.pointerType !== 'mouse' && event.pointerType !== 'pen' && event.pointerType !== 'touch') return null;

    const scrollSurface = element.closest<HTMLElement>('[role="tree"]');
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
    };
    this.#session = session;

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
    this.ghost = { ...session.item, x: event.clientX, y: event.clientY, width: session.element.offsetWidth };
    this.#paneGroup.updateActiveZone(event.clientX, event.clientY);
  }

  #end(session: PointerSession, event: PointerEvent): void {
    if (!session.active) {
      if (session.touchHoldCanceled) this.#armClickSuppression(session);
      this.#finish(session, { cancelPane: false, suppressClick: false });
      this.#expireClickSuppressionAfterPointerUp(event.pointerId);
      return;
    }

    this.#paneGroup.updateActiveZone(event.clientX, event.clientY);
    const hasDropZone = this.#paneGroup.activeZone !== null;
    const succeeded = hasDropZone && this.#paneGroup.executeDrop({ slug: session.item.slug, type: 'document' });
    this.#finish(session, { cancelPane: !succeeded, suppressClick: true });
    this.#expireClickSuppressionAfterPointerUp(event.pointerId);
    if (succeeded) this.#onDropSuccess();
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
