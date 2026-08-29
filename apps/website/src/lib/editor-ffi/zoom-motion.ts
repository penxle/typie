export const ZOOM_ELASTIC_EXTENT_RATIO = 1.25;
export const ZOOM_ELASTIC_RESISTANCE = 0.55;
export const ZOOM_MAX_MOTION_SECONDS = 0.24;
export const ZOOM_SPRING_ANGULAR_FREQUENCY = 24;
export const ZOOM_SETTLE_EPSILON = 0.0005;

export type ZoomMotionBounds = { min: number; max: number };

export type ZoomMotionFrame = {
  displayZoom: number;
  finished: boolean;
};

export function elasticDisplayZoom(rawZoom: number, bounds: ZoomMotionBounds): number | null {
  if (!isValidZoom(rawZoom) || !isValidBounds(bounds)) return null;

  const rawLog = Math.log(rawZoom);
  const minimumLog = Math.log(bounds.min);
  const maximumLog = Math.log(bounds.max);
  const elasticExtent = Math.log(ZOOM_ELASTIC_EXTENT_RATIO);
  const displayLog =
    rawLog > maximumLog
      ? maximumLog + rubberBandDistance(rawLog - maximumLog, elasticExtent)
      : rawLog < minimumLog
        ? minimumLog - rubberBandDistance(minimumLog - rawLog, elasticExtent)
        : rawLog;
  return Math.exp(displayLog);
}

export class ZoomMotion {
  #elapsedSeconds = 0;
  #frame: ZoomMotionFrame;
  #initialLogZoom: number;
  #targetLogZoom: number;

  constructor(displayZoom: number, bounds: ZoomMotionBounds) {
    if (!isValidZoom(displayZoom) || !isValidBounds(bounds) || (displayZoom >= bounds.min && displayZoom <= bounds.max)) {
      throw new Error('Zoom recovery requires an out-of-bounds display zoom');
    }

    this.#initialLogZoom = Math.log(displayZoom);
    this.#targetLogZoom = Math.log(displayZoom < bounds.min ? bounds.min : bounds.max);
    this.#frame = {
      displayZoom,
      finished: false,
    };
  }

  #springLogZoom(elapsed: number): number {
    const displacement = this.#initialLogZoom - this.#targetLogZoom;
    const angularFrequency = ZOOM_SPRING_ANGULAR_FREQUENCY;
    const decay = Math.exp(-angularFrequency * elapsed);
    return this.#targetLogZoom + displacement * (1 + angularFrequency * elapsed) * decay;
  }

  #isSettled(logZoom: number): boolean {
    return Math.abs(logZoom - this.#targetLogZoom) <= ZOOM_SETTLE_EPSILON;
  }

  advance(deltaSeconds: number): ZoomMotionFrame {
    if (!Number.isFinite(deltaSeconds) || deltaSeconds <= 0 || this.#frame.finished) return this.#frame;
    this.#elapsedSeconds += deltaSeconds;
    const logZoom = this.#springLogZoom(this.#elapsedSeconds);
    const finished = this.#elapsedSeconds >= ZOOM_MAX_MOTION_SECONDS || this.#isSettled(logZoom);
    this.#frame = {
      displayZoom: Math.exp(finished ? this.#targetLogZoom : logZoom),
      finished,
    };
    return this.#frame;
  }
}

function rubberBandDistance(distance: number, extent: number): number {
  return (extent * ZOOM_ELASTIC_RESISTANCE * distance) / (extent + ZOOM_ELASTIC_RESISTANCE * distance);
}

function isValidZoom(value: number): boolean {
  return Number.isFinite(value) && value > 0;
}

function isValidBounds(bounds: ZoomMotionBounds): boolean {
  return isValidZoom(bounds.min) && isValidZoom(bounds.max) && bounds.max >= bounds.min;
}
