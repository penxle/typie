import { pointerCapture } from './pointer-capture.svelte';
import type { ActionReturn } from 'svelte/action';
import type { PointerCaptureCancelReason } from './pointer-capture.svelte';

export type ThresholdDragControls = {
  cancel: () => void;
};

export type ThresholdDragParameters<Session> = {
  threshold?: number;
  start: (event: PointerEvent) => Session | null;
  activate: (session: Session, startEvent: PointerEvent, event: PointerEvent, controls: ThresholdDragControls) => boolean;
  move?: (session: Session, event: PointerEvent) => void;
  press?: (session: Session, event: PointerEvent) => void;
  end?: (session: Session, event: PointerEvent) => void;
  cancel?: (session: Session, reason: PointerCaptureCancelReason, wasActive: boolean, event?: PointerEvent) => void;
};

export type ThresholdDragHandle<Session> = ActionReturn<ThresholdDragParameters<Session>> & {
  cancel: () => void;
  destroy: () => void;
  state: () => { active: boolean };
};

type DragSession<Session> = {
  value: Session;
  startEvent: PointerEvent;
  active: boolean;
  rejected: boolean;
  ended: boolean;
  frame: number | null;
  pendingMove: PointerEvent | null;
};

export const thresholdDrag = <Session>(
  element: HTMLElement,
  initialParameters: ThresholdDragParameters<Session>,
  captureTarget = element,
): ThresholdDragHandle<Session> => {
  let parameters = initialParameters;
  let current: DragSession<Session> | null = null;

  const finish = (session: DragSession<Session>) => {
    if (current !== session || session.ended) return false;

    session.ended = true;
    if (session.frame !== null) {
      cancelAnimationFrame(session.frame);
      session.frame = null;
    }
    session.pendingMove = null;
    current = null;
    return true;
  };

  const processMove = (session: DragSession<Session>, event: PointerEvent) => {
    if (current !== session || session.ended) return;

    if (!session.active) {
      const distance = Math.hypot(event.clientX - session.startEvent.clientX, event.clientY - session.startEvent.clientY);
      if (distance <= (parameters.threshold ?? 10)) return;

      const controls: ThresholdDragControls = {
        cancel: () => {
          if (current === session && !session.ended) capture.cancel();
        },
      };
      const accepted = parameters.activate(session.value, session.startEvent, event, controls);
      if (current !== session || session.ended) return;
      if (!accepted) {
        session.rejected = true;
        capture.cancel();
        finish(session);
        return;
      }
      session.active = true;
    }

    parameters.move?.(session.value, event);
  };

  const flushMove = (session: DragSession<Session>) => {
    if (session.frame !== null) {
      cancelAnimationFrame(session.frame);
      session.frame = null;
    }
    const event = session.pendingMove;
    session.pendingMove = null;
    if (event) processMove(session, event);
  };

  const capture = pointerCapture<DragSession<Session>>(
    element,
    {
      start: (event) => {
        const value = parameters.start(event);
        if (value === null) return null;

        const session: DragSession<Session> = {
          value,
          startEvent: event,
          active: false,
          rejected: false,
          ended: false,
          frame: null,
          pendingMove: null,
        };
        current = session;
        return session;
      },
      move: (session, event) => {
        session.pendingMove = event;
        if (session.frame !== null) return;

        session.frame = requestAnimationFrame(() => {
          if (current !== session || session.ended) return;
          session.frame = null;
          const pendingMove = session.pendingMove;
          session.pendingMove = null;
          if (pendingMove) processMove(session, pendingMove);
        });
      },
      end: (session, event) => {
        flushMove(session);
        if (current !== session || session.ended) return;

        const wasActive = session.active;
        const value = session.value;
        finish(session);
        if (wasActive) {
          parameters.end?.(value, event);
        } else {
          parameters.press?.(value, event);
        }
      },
      cancel: (session, reason, event) => {
        const wasActive = session.active;
        const rejected = session.rejected;
        const value = session.value;
        const finished = finish(session);
        if (!finished || rejected) return;
        parameters.cancel?.(value, reason, wasActive, event);
      },
    },
    captureTarget,
  );

  return {
    cancel: capture.cancel,
    update(nextParameters) {
      parameters = nextParameters;
    },
    destroy: capture.destroy,
    state: () => ({ active: current?.active ?? false }),
  };
};
