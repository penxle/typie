import type { Action } from 'svelte/action';

export type HoverIntentParameter = {
  /** Maximum time to wait before intent is assumed. */
  delay: number;
  /** Enables intent detection without suppressing the enter/leave lifecycle. */
  intentEnabled?: boolean;
  /** Maximum movement in pixels per 100ms that counts as low-speed movement. */
  sensitivity?: number;
  /** Consecutive low-speed samples required before intent is established. */
  samples?: number;
  onEnter?: (event: PointerEvent) => void;
  onIntent: (event: PointerEvent) => void;
  onLeave?: (event: PointerEvent) => void;
};

const SAMPLE_INTERVAL_MS = 100;
const DEFAULT_SENSITIVITY = 6;
const DEFAULT_SAMPLES = 2;

const resolveSamples = (value: number | undefined) => Math.max(1, Math.floor(value ?? DEFAULT_SAMPLES));

export const hoverIntent: Action<HTMLElement, HoverIntentParameter> = (element, parameter) => {
  let current = parameter;
  let fallbackTimer: ReturnType<typeof setTimeout> | undefined;
  let sampleTimer: ReturnType<typeof setTimeout> | undefined;
  let hovered = false;
  let intended = false;
  let pointerX = 0;
  let pointerY = 0;
  let previousX = 0;
  let previousY = 0;
  let previousSampleTime = 0;
  let lowSpeedSamples = 0;
  let pointerEvent: PointerEvent | undefined;

  const clearDetection = () => {
    if (fallbackTimer !== undefined) clearTimeout(fallbackTimer);
    if (sampleTimer !== undefined) clearTimeout(sampleTimer);
    fallbackTimer = undefined;
    sampleTimer = undefined;
    lowSpeedSamples = 0;
  };

  const fireIntent = (event: PointerEvent) => {
    clearDetection();
    intended = true;
    current.onIntent(event);
  };

  const sample = () => {
    sampleTimer = undefined;
    if (!pointerEvent || !hovered || intended || current.intentEnabled === false) return;

    const now = performance.now();
    const elapsed = Math.max(1, now - previousSampleTime);
    const distance = Math.hypot(pointerX - previousX, pointerY - previousY);
    const normalizedDistance = (distance * SAMPLE_INTERVAL_MS) / elapsed;
    lowSpeedSamples = normalizedDistance < (current.sensitivity ?? DEFAULT_SENSITIVITY) ? lowSpeedSamples + 1 : 0;
    previousX = pointerX;
    previousY = pointerY;
    previousSampleTime = now;

    if (lowSpeedSamples >= resolveSamples(current.samples)) fireIntent(pointerEvent);
    else sampleTimer = setTimeout(sample, SAMPLE_INTERVAL_MS);
  };

  const startDetection = (event: PointerEvent) => {
    clearDetection();
    pointerEvent = event;
    pointerX = event.clientX;
    pointerY = event.clientY;
    previousX = pointerX;
    previousY = pointerY;
    previousSampleTime = performance.now();
    if (current.intentEnabled === false) return;
    if (current.delay <= 0) {
      fireIntent(event);
      return;
    }

    fallbackTimer = setTimeout(() => {
      fallbackTimer = undefined;
      if (pointerEvent) fireIntent(pointerEvent);
    }, current.delay);
    sampleTimer = setTimeout(sample, SAMPLE_INTERVAL_MS);
  };

  const handlePointerEnter = (event: PointerEvent) => {
    if (event.pointerType === 'touch') return;
    hovered = true;
    intended = false;
    pointerEvent = event;
    pointerX = event.clientX;
    pointerY = event.clientY;
    current.onEnter?.(event);
    startDetection(event);
  };

  const handlePointerMove = (event: PointerEvent) => {
    if (intended || event.pointerType === 'touch') return;
    if (!hovered) {
      handlePointerEnter(event);
      return;
    }
    pointerEvent = event;
    pointerX = event.clientX;
    pointerY = event.clientY;
  };

  const endSession = (event: PointerEvent) => {
    if (!hovered || event.pointerType === 'touch') return;
    clearDetection();
    hovered = false;
    intended = false;
    current.onLeave?.(event);
  };

  element.addEventListener('pointerenter', handlePointerEnter);
  element.addEventListener('pointermove', handlePointerMove);
  element.addEventListener('pointerleave', endSession);
  element.addEventListener('pointercancel', endSession);

  return {
    update: (next) => {
      const intentWasEnabled = current.intentEnabled !== false;
      const delayChanged = next.delay !== current.delay;
      const sensitivityChanged = next.sensitivity !== current.sensitivity;
      const samplesChanged = next.samples !== current.samples;
      current = next;
      const intentIsEnabled = current.intentEnabled !== false;
      if (!intentIsEnabled) {
        clearDetection();
        intended = false;
      } else if (hovered && !intended && pointerEvent && (!intentWasEnabled || delayChanged || sensitivityChanged || samplesChanged)) {
        startDetection(pointerEvent);
      }
    },
    destroy: () => {
      clearDetection();
      hovered = false;
      intended = false;
      element.removeEventListener('pointerenter', handlePointerEnter);
      element.removeEventListener('pointermove', handlePointerMove);
      element.removeEventListener('pointerleave', endSession);
      element.removeEventListener('pointercancel', endSession);
    },
  };
};
