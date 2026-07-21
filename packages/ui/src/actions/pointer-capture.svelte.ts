import type { ActionReturn } from 'svelte/action';

export type PointerCaptureCancelReason = 'pointercancel' | 'lostpointercapture' | 'capture-failed' | 'programmatic' | 'destroy';

export type PointerCaptureParameters<Session> = {
  start: (event: PointerEvent) => Session | null;
  move?: (session: Session, event: PointerEvent) => void;
  end?: (session: Session, event: PointerEvent) => void;
  cancel?: (session: Session, reason: PointerCaptureCancelReason, event?: PointerEvent) => void;
};

type ActiveSession<Session> = {
  pointerId: number;
  value: Session;
};

export type PointerCaptureHandle<Session> = ActionReturn<PointerCaptureParameters<Session>> & {
  cancel: () => void;
  destroy: () => void;
};

export const pointerCapture = <Session>(
  element: HTMLElement,
  initialParameters: PointerCaptureParameters<Session>,
): PointerCaptureHandle<Session> => {
  let parameters = initialParameters;
  let active: ActiveSession<Session> | null = null;

  const release = (pointerId: number) => {
    if (element.hasPointerCapture(pointerId)) {
      element.releasePointerCapture(pointerId);
    }
  };

  const cancel = (reason: PointerCaptureCancelReason, event?: PointerEvent) => {
    const current = active;
    if (!current || (event && event.pointerId !== current.pointerId)) return;

    active = null;
    if (reason !== 'lostpointercapture') {
      release(current.pointerId);
    }
    parameters.cancel?.(current.value, reason, event);
  };

  const handlePointerDown = (event: PointerEvent) => {
    if (active) return;

    const value = parameters.start(event);
    if (value === null) return;

    active = { pointerId: event.pointerId, value };
    try {
      element.setPointerCapture(event.pointerId);
    } catch {
      cancel('capture-failed', event);
    }
  };

  const handlePointerMove = (event: PointerEvent) => {
    const current = active;
    if (!current || event.pointerId !== current.pointerId) return;
    parameters.move?.(current.value, event);
  };

  const handlePointerUp = (event: PointerEvent) => {
    const current = active;
    if (!current || event.pointerId !== current.pointerId) return;

    active = null;
    release(current.pointerId);
    parameters.end?.(current.value, event);
  };

  const handlePointerCancel = (event: PointerEvent) => cancel('pointercancel', event);
  const handleLostPointerCapture = (event: PointerEvent) => cancel('lostpointercapture', event);

  element.addEventListener('pointerdown', handlePointerDown);
  element.addEventListener('pointermove', handlePointerMove);
  element.addEventListener('pointerup', handlePointerUp);
  element.addEventListener('pointercancel', handlePointerCancel);
  element.addEventListener('lostpointercapture', handleLostPointerCapture);

  return {
    cancel() {
      cancel('programmatic');
    },
    update(nextParameters) {
      parameters = nextParameters;
    },
    destroy() {
      element.removeEventListener('pointerdown', handlePointerDown);
      element.removeEventListener('pointermove', handlePointerMove);
      element.removeEventListener('pointerup', handlePointerUp);
      element.removeEventListener('pointercancel', handlePointerCancel);
      element.removeEventListener('lostpointercapture', handleLostPointerCapture);
      cancel('destroy');
    },
  };
};
