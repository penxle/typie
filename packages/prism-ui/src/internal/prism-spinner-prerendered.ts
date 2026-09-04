/* eslint-disable unicorn/consistent-class-member-order -- Public playback lifecycle methods stay grouped; private loading helpers remain beside their call sites. */

import { normalizeSpinnerPhase, spinnerFrameIndexForPhase, spinnerPhaseForFrameIndex } from './prism-spinner-morph.ts';

export type PrismSpinnerAtlasConfiguration = {
  atlasUrl: string;
  columns: number;
  cssSize: number;
  durationMs: number;
  frameCount: number;
  framePixelSize: number;
  rows: number;
};

type SpinnerAnimationFrameSchedulerOptions = {
  cancelFrame: (frameId: number) => void;
  requestFrame: (callback: FrameRequestCallback) => number;
};

type SpinnerAnimationFrameScheduler = {
  subscribe: (subscriber: FrameRequestCallback) => () => void;
};

export function createSpinnerAnimationFrameScheduler({
  cancelFrame,
  requestFrame,
}: SpinnerAnimationFrameSchedulerOptions): SpinnerAnimationFrameScheduler {
  const subscribers = new Set<FrameRequestCallback>();
  let frameId: number | null = null;

  const schedule = (): void => {
    if (frameId !== null || subscribers.size === 0) return;
    frameId = requestFrame((now) => {
      frameId = null;
      for (const subscriber of subscribers) subscriber(now);
      schedule();
    });
  };

  return {
    subscribe(subscriber) {
      subscribers.add(subscriber);
      schedule();
      let subscribed = true;
      return () => {
        if (!subscribed) return;
        subscribed = false;
        subscribers.delete(subscriber);
        if (frameId !== null && subscribers.size === 0) {
          cancelFrame(frameId);
          frameId = null;
        }
      };
    },
  };
}

export function shouldScheduleSpinnerAtlasPlayback(state: { atlasReady: boolean; playing: boolean; visible: boolean }): boolean {
  return state.atlasReady && state.playing && state.visible;
}

const spinnerAnimationFrameScheduler = createSpinnerAnimationFrameScheduler({
  cancelFrame: (frameId) => cancelAnimationFrame(frameId),
  requestFrame: (callback) => requestAnimationFrame(callback),
});

export function createSpinnerFramePlayback(durationMs: number, frameCount: number) {
  if (!Number.isFinite(durationMs) || durationMs <= 0) throw new RangeError('Spinner duration must be positive.');
  if (!Number.isSafeInteger(frameCount) || frameCount <= 0) throw new RangeError('Spinner frame count must be positive.');
  let heldPhase = 0;
  let startedAt: number | null = null;
  let lastFrameIndex = -1;

  function phaseAt(now = performance.now()): number {
    return startedAt === null ? heldPhase : normalizeSpinnerPhase((now - startedAt) / durationMs);
  }

  return {
    frameIndexAt(now = performance.now()) {
      return spinnerFrameIndexForPhase(phaseAt(now), frameCount);
    },
    pause(now = performance.now()) {
      const frameIndex = spinnerFrameIndexForPhase(phaseAt(now), frameCount);
      heldPhase = spinnerPhaseForFrameIndex(frameIndex, frameCount);
      startedAt = null;
    },
    phaseAt,
    playFromPhase(phase: number, now = performance.now()) {
      heldPhase = normalizeSpinnerPhase(phase);
      startedAt = now - heldPhase * durationMs;
      lastFrameIndex = -1;
    },
    setPhase(phase: number) {
      const frameIndex = spinnerFrameIndexForPhase(phase, frameCount);
      heldPhase = spinnerPhaseForFrameIndex(frameIndex, frameCount);
      startedAt = null;
      lastFrameIndex = -1;
      return frameIndex;
    },
    takeFrame(now = performance.now()) {
      const frameIndex = spinnerFrameIndexForPhase(phaseAt(now), frameCount);
      if (frameIndex === lastFrameIndex) return null;
      lastFrameIndex = frameIndex;
      return frameIndex;
    },
  };
}

export class PrismSpinnerPreRenderedPlayer {
  private atlas: HTMLImageElement | null = null;
  private configuration: PrismSpinnerAtlasConfiguration | null = null;
  private context: CanvasRenderingContext2D;
  private playing = false;
  private playback = createSpinnerFramePlayback(2300, 138);
  private readyPromise: Promise<void> = Promise.reject(new Error('Spinner atlas is not connected.'));
  private state: 'failed' | 'loading' | 'ready' = 'loading';
  private unsubscribeFrame: (() => void) | null = null;
  private visible = true;
  private visibilityObserver: IntersectionObserver | null = null;
  private readonly canvas: HTMLCanvasElement;

  constructor(canvas: HTMLCanvasElement) {
    this.canvas = canvas;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('A 2D canvas context is unavailable.');
    this.context = context;
    this.observeVisibility();
    this.readyPromise.catch(() => {
      // Callers observe readiness through the public promise and state.
    });
  }

  connect(configuration: PrismSpinnerAtlasConfiguration): Promise<void> {
    this.observeVisibility();
    this.pause();
    this.configuration = configuration;
    this.playback = createSpinnerFramePlayback(configuration.durationMs, configuration.frameCount);
    this.state = 'loading';
    this.canvas.width = configuration.framePixelSize;
    this.canvas.height = configuration.framePixelSize;
    this.canvas.style.width = `${configuration.cssSize}px`;
    this.canvas.style.height = `${configuration.cssSize}px`;
    const generationConfiguration = configuration;
    this.readyPromise = this.loadAtlas(configuration)
      .then((atlas) => {
        if (this.configuration !== generationConfiguration) return;
        this.atlas = atlas;
        this.state = 'ready';
        this.setPhase(0);
      })
      .catch((err) => {
        if (this.configuration === generationConfiguration) this.state = 'failed';
        throw err;
      });
    this.readyPromise.catch(() => {
      // Callers observe readiness through the public promise and state.
    });
    return this.readyPromise;
  }

  get readiness(): 'failed' | 'loading' | 'ready' {
    return this.state;
  }

  setPhase(phase: number): number {
    this.playing = false;
    this.syncFrameSubscription();
    const frameIndex = this.playback.setPhase(phase);
    this.drawFrame(frameIndex);
    return frameIndex;
  }

  playFromPhase(phase: number, now = performance.now()): void {
    if (this.state !== 'ready') return;
    this.playing = true;
    this.playback.playFromPhase(phase, now);
    this.syncFrameSubscription();
  }

  phaseAt(now = performance.now()): number {
    return this.playback.phaseAt(now);
  }

  frameIndexAt(now = performance.now()): number {
    return this.playback.frameIndexAt(now);
  }

  pause(now = performance.now()): void {
    this.playing = false;
    this.syncFrameSubscription();
    this.playback.pause(now);
    const frameIndex = this.playback.takeFrame(now);
    if (frameIndex !== null) this.drawFrame(frameIndex);
  }

  dispose(): void {
    this.unsubscribeFromFrames();
    this.atlas = null;
    this.configuration = null;
    this.playing = false;
    this.visibilityObserver?.disconnect();
    this.visibilityObserver = null;
    this.context.clearRect(0, 0, this.canvas.width, this.canvas.height);
  }

  private async loadAtlas(configuration: PrismSpinnerAtlasConfiguration): Promise<HTMLImageElement> {
    const atlas = new Image();
    atlas.decoding = 'async';
    atlas.src = configuration.atlasUrl;
    await atlas.decode();
    const expectedWidth = configuration.columns * configuration.framePixelSize;
    const expectedHeight = configuration.rows * configuration.framePixelSize;
    if (atlas.naturalWidth !== expectedWidth || atlas.naturalHeight !== expectedHeight) {
      throw new RangeError(`Spinner atlas is ${atlas.naturalWidth}x${atlas.naturalHeight}; expected ${expectedWidth}x${expectedHeight}.`);
    }
    return atlas;
  }

  private readonly handleScheduledFrame = (now: number): void => {
    if (this.state !== 'ready') return;
    const frameIndex = this.playback.takeFrame(now);
    if (frameIndex !== null) this.drawFrame(frameIndex);
  };

  private drawFrame(frameIndex: number): void {
    const atlas = this.atlas;
    const configuration = this.configuration;
    if (!atlas || !configuration) return;
    const sourceX = (frameIndex % configuration.columns) * configuration.framePixelSize;
    const sourceY = Math.floor(frameIndex / configuration.columns) * configuration.framePixelSize;
    this.context.clearRect(0, 0, configuration.framePixelSize, configuration.framePixelSize);
    this.context.drawImage(
      atlas,
      sourceX,
      sourceY,
      configuration.framePixelSize,
      configuration.framePixelSize,
      0,
      0,
      configuration.framePixelSize,
      configuration.framePixelSize,
    );
  }

  private syncFrameSubscription(): void {
    const shouldSubscribe = shouldScheduleSpinnerAtlasPlayback({
      atlasReady: this.state === 'ready',
      playing: this.playing,
      visible: this.visible,
    });
    if (shouldSubscribe && !this.unsubscribeFrame) {
      this.unsubscribeFrame = spinnerAnimationFrameScheduler.subscribe(this.handleScheduledFrame);
    } else if (!shouldSubscribe) {
      this.unsubscribeFromFrames();
    }
  }

  private observeVisibility(): void {
    if (this.visibilityObserver || !globalThis.IntersectionObserver) return;
    this.visible = false;
    this.visibilityObserver = new IntersectionObserver(([entry]) => {
      this.visible = entry?.isIntersecting ?? false;
      this.syncFrameSubscription();
    });
    this.visibilityObserver.observe(this.canvas);
  }

  private unsubscribeFromFrames(): void {
    this.unsubscribeFrame?.();
    this.unsubscribeFrame = null;
  }
}
