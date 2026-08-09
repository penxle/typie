const MIN_SETTLE_SECONDS = 0.18;
const DISTANCE_SETTLE_SECONDS = 0.11;
const MAX_SETTLE_SECONDS = 0.65;
const ONE_PERCENT_RESPONSE = 6.64;
const POSITION_THRESHOLD = 0.5;
const VELOCITY_THRESHOLD = 5;

export type SmoothScrollMotionSnapshot = {
  position: number;
  velocity: number;
  target: number;
};

type SmoothScrollMotionStart = {
  position: number;
  target: number;
  viewportHeight: number;
};

export class SmoothScrollMotion {
  static start({ position, target, viewportHeight }: SmoothScrollMotionStart): SmoothScrollMotion {
    return new SmoothScrollMotion(position, target, viewportHeight);
  }

  #position: number;
  #velocity = 0;
  #target: number;
  #omega: number;
  #finished = false;

  private constructor(position: number, target: number, viewportHeight: number) {
    this.#position = position;
    this.#target = target;
    this.#omega = responseRate(position, target, viewportHeight);
    this.#finishIfArrived();
  }

  #finishIfArrived(): void {
    if (Math.abs(this.#target - this.#position) <= POSITION_THRESHOLD && Math.abs(this.#velocity) <= VELOCITY_THRESHOLD) {
      this.#finish();
    }
  }

  #finish(): void {
    this.#position = this.#target;
    this.#velocity = 0;
    this.#finished = true;
  }

  get finished(): boolean {
    return this.#finished;
  }

  advance(deltaSeconds: number): SmoothScrollMotionSnapshot {
    if (this.#finished || !Number.isFinite(deltaSeconds) || deltaSeconds <= 0) {
      return this.snapshot();
    }
    const previousRemaining = this.#target - this.#position;
    const error = this.#position - this.#target;
    const b = this.#velocity + this.#omega * error;
    const decay = Math.exp(-this.#omega * deltaSeconds);
    this.#position = this.#target + (error + b * deltaSeconds) * decay;
    this.#velocity = (this.#velocity - this.#omega * b * deltaSeconds) * decay;
    if (previousRemaining * (this.#target - this.#position) < 0) {
      this.#finish();
    } else {
      this.#finishIfArrived();
    }
    return this.snapshot();
  }

  retarget(target: number, viewportHeight: number): void {
    if (!Number.isFinite(target)) return;
    this.#target = target;
    if (Math.abs(this.#target - this.#position) <= POSITION_THRESHOLD) {
      this.#finish();
      return;
    }
    this.#finished = false;
    const remaining = this.#target - this.#position;
    const towardVelocity = this.#velocity * Math.sign(remaining);
    this.#omega = Math.max(responseRate(this.#position, this.#target, viewportHeight), Math.max(0, towardVelocity) / Math.abs(remaining));
  }

  translate(delta: number): void {
    if (!Number.isFinite(delta)) return;
    this.#position += delta;
    this.#target += delta;
  }

  synchronizeBounds(actualPosition: number, maximumScroll: number, viewportHeight: number): void {
    if (!Number.isFinite(actualPosition) || !Number.isFinite(maximumScroll)) return;
    const maximum = Math.max(0, maximumScroll);
    this.#position = Math.max(0, Math.min(actualPosition, maximum));
    this.#target = Math.max(0, Math.min(this.#target, maximum));
    if ((this.#position <= 0 && this.#velocity < 0) || (this.#position >= maximum && this.#velocity > 0)) {
      this.#velocity = 0;
    }
    this.retarget(this.#target, viewportHeight);
  }

  cancel(): void {
    this.#target = this.#position;
    this.#finish();
  }

  snapshot(): SmoothScrollMotionSnapshot {
    return { position: this.#position, velocity: this.#velocity, target: this.#target };
  }
}

function responseRate(position: number, target: number, viewportHeight: number): number {
  const distance = Math.abs(target - position) / Math.max(1, viewportHeight);
  const settleSeconds = Math.min(MAX_SETTLE_SECONDS, MIN_SETTLE_SECONDS + DISTANCE_SETTLE_SECONDS * Math.sqrt(distance));
  return ONE_PERCENT_RESPONSE / settleSeconds;
}
