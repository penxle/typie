import type { ActionReturn } from 'svelte/action';

export type PointerCaptureCancelReason = 'pointercancel' | 'lostpointercapture' | 'capture-failed' | 'programmatic' | 'destroy';

export type PointerCaptureParameters<Session> = {
  start: (event: PointerEvent) => Session | null;
  move?: (session: Session, event: PointerEvent) => void;
  end?: (session: Session, event: PointerEvent) => void;
  cancel?: (session: Session, reason: PointerCaptureCancelReason, event?: PointerEvent) => void;
};

type ActiveSession<Session> = {
  captureTarget: HTMLElement;
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
  captureTarget = element,
): PointerCaptureHandle<Session> => {
  let parameters = initialParameters;
  let active: ActiveSession<Session> | null = null;
  let listening = false;

  const release = (session: ActiveSession<Session>) => {
    if (session.captureTarget.hasPointerCapture(session.pointerId)) {
      session.captureTarget.releasePointerCapture(session.pointerId);
    }
  };

  const stopListening = () => {
    if (!listening) return;
    listening = false;
    captureTarget.removeEventListener('pointermove', handlePointerMove);
    captureTarget.removeEventListener('pointerup', handlePointerUp);
    captureTarget.removeEventListener('pointercancel', handlePointerCancel);
    captureTarget.removeEventListener('lostpointercapture', handleLostPointerCapture);
  };

  const cancel = (reason: PointerCaptureCancelReason, event?: PointerEvent) => {
    const current = active;
    if (!current || (event && event.pointerId !== current.pointerId)) return;

    active = null;
    if (reason !== 'lostpointercapture') {
      release(current);
    }
    stopListening();
    parameters.cancel?.(current.value, reason, event);
  };

  const handlePointerDown = (event: PointerEvent) => {
    if (active) return;

    const value = parameters.start(event);
    if (value === null) return;

    active = { captureTarget, pointerId: event.pointerId, value };
    listening = true;
    captureTarget.addEventListener('pointermove', handlePointerMove);
    captureTarget.addEventListener('pointerup', handlePointerUp);
    captureTarget.addEventListener('pointercancel', handlePointerCancel);
    captureTarget.addEventListener('lostpointercapture', handleLostPointerCapture);
    try {
      captureTarget.setPointerCapture(event.pointerId);
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
    release(current);
    stopListening();
    parameters.end?.(current.value, event);
  };

  const handlePointerCancel = (event: PointerEvent) => cancel('pointercancel', event);
  const handleLostPointerCapture = (event: PointerEvent) => cancel('lostpointercapture', event);

  element.addEventListener('pointerdown', handlePointerDown);

  return {
    cancel() {
      cancel('programmatic');
    },
    update(nextParameters) {
      parameters = nextParameters;
    },
    destroy() {
      element.removeEventListener('pointerdown', handlePointerDown);
      cancel('destroy');
      stopListening();
    },
  };
};
