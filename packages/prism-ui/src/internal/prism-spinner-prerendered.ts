/* eslint-disable unicorn/consistent-class-member-order -- Public playback lifecycle methods stay grouped; private loading helpers remain beside their call sites. */

import { flushPrismSpinnerHdrRenderBatch, PrismSpinnerHdrOverlay } from './prism-spinner-hdr.ts';
import { normalizeSpinnerPhase, spinnerFrameIndexForPhase, spinnerPhaseForFrameIndex } from './prism-spinner-morph.ts';
import { decodeSpinnerHdrAsset, interpolateSpinnerHdrFrame } from './spinner-hdr-asset.ts';
import type { PrismSpinnerHdrMode, PrismSpinnerHdrState } from './prism-spinner-hdr.ts';
import type { SpinnerHdrAsset } from './spinner-hdr-asset.ts';

export type PrismSpinnerPreRenderedHdrConfiguration = {
  assetUrl: string;
  cssSize: number;
  durationMs: number;
  frameCount: number;
  headroom: number;
  mode: PrismSpinnerHdrMode;
};

export type PrismSpinnerAtlasConfiguration = {
  atlasUrl: string;
  columns: number;
  cssSize: number;
  durationMs: number;
  frameCount: number;
  framePixelSize: number;
  hdr?: PrismSpinnerPreRenderedHdrConfiguration;
  rows: number;
};

const assetCache = new Map<string, Promise<SpinnerHdrAsset>>();

type SpinnerAnimationFrameSchedulerOptions = {
  afterFrame: () => void;
  cancelFrame: (frameId: number) => void;
  requestFrame: (callback: FrameRequestCallback) => number;
};

type SpinnerAnimationFrameScheduler = {
  subscribe: (subscriber: FrameRequestCallback) => () => void;
};

export function createSpinnerAnimationFrameScheduler({
  afterFrame,
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
      afterFrame();
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

export function shouldScheduleSpinnerHdrPlayback(state: {
  assetReady: boolean;
  hdrActive: boolean;
  playing: boolean;
  visible: boolean;
}): boolean {
  return state.assetReady && state.hdrActive && state.playing && state.visible;
}

export function shouldScheduleSpinnerAtlasPlayback(state: { atlasReady: boolean; playing: boolean; visible: boolean }): boolean {
  return state.atlasReady && state.playing && state.visible;
}

const spinnerAnimationFrameScheduler = createSpinnerAnimationFrameScheduler({
  afterFrame: flushPrismSpinnerHdrRenderBatch,
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

function loadAsset(url: string): Promise<SpinnerHdrAsset> {
  const cached = assetCache.get(url);
  if (cached) return cached;
  const request = fetch(url).then(async (response) => {
    if (!response.ok) throw new Error(`Unable to load spinner HDR asset: HTTP ${response.status}.`);
    return decodeSpinnerHdrAsset(await response.arrayBuffer());
  });
  assetCache.set(url, request);
  request.catch(() => assetCache.delete(url));
  return request;
}

export class PrismSpinnerPreRenderedHdrPlayer {
  private asset: SpinnerHdrAsset | null = null;
  private configuration: PrismSpinnerPreRenderedHdrConfiguration | null = null;
  private generation = 0;
  private hdrState: PrismSpinnerHdrState = 'off';
  private interpolatedVertices: Float32Array | undefined;
  private lastFrameIndex = -1;
  private overlay: PrismSpinnerHdrOverlay;
  private unsubscribeFrame: (() => void) | null = null;
  private visible = true;
  private visibilityObserver: IntersectionObserver | null = null;
  private startedAt: number | null = null;
  private heldPhase = 0;
  private readiness: Promise<PrismSpinnerHdrState> = Promise.resolve('off');
  private resolveReadiness: ((state: PrismSpinnerHdrState) => void) | null = null;
  private readonly canvas: HTMLCanvasElement;
  private readonly onStateChange: (state: PrismSpinnerHdrState) => void;
  private readonly handleHdrStateChange = (state: PrismSpinnerHdrState): void => {
    this.hdrState = state;
    this.onStateChange(state);
    if (state === 'active') void this.load(this.generation);
    else {
      this.unsubscribeFromFrames();
      if (state === 'failed' || state === 'off' || state === 'unsupported') this.settleReadiness(state);
    }
    this.syncFrameSubscription();
  };

  constructor(
    canvas: HTMLCanvasElement,
    onStateChange: (state: PrismSpinnerHdrState) => void = () => {
      // State observation is optional for the standalone player.
    },
  ) {
    this.canvas = canvas;
    this.onStateChange = onStateChange;
    this.overlay = new PrismSpinnerHdrOverlay(canvas, this.handleHdrStateChange);
    this.observeVisibility();
  }

  connect(configuration: PrismSpinnerPreRenderedHdrConfiguration): void {
    this.observeVisibility();
    const sourceChanged = this.configuration?.assetUrl !== configuration.assetUrl;
    const appearanceChanged =
      this.configuration?.headroom !== configuration.headroom || this.configuration?.cssSize !== configuration.cssSize;
    this.configuration = configuration;
    this.readiness = new Promise((resolve) => {
      this.resolveReadiness = resolve;
    });
    this.canvas.style.width = `${configuration.cssSize}px`;
    this.canvas.style.height = `${configuration.cssSize}px`;
    if (sourceChanged) {
      this.asset = null;
      this.startedAt = null;
      this.interpolatedVertices = undefined;
      this.lastFrameIndex = -1;
      this.generation += 1;
      this.unsubscribeFromFrames();
      this.overlay.renderVertices(configuration.cssSize, new Float32Array());
    } else if (appearanceChanged) {
      this.lastFrameIndex = -1;
    }
    this.overlay.connect(configuration.mode);
    if (configuration.mode === 'off') this.settleReadiness('off');
    if (this.hdrState === 'active') {
      if (!this.asset) void this.load(this.generation);
      else if (appearanceChanged && this.startedAt !== null) {
        this.syncFrameSubscription();
      }
    }
    this.syncFrameSubscription();
  }

  disconnect(): void {
    this.generation += 1;
    this.unsubscribeFromFrames();
    this.visibilityObserver?.disconnect();
    this.visibilityObserver = null;
    this.visible = !globalThis.IntersectionObserver;
    this.startedAt = null;
    this.configuration = null;
    this.asset = null;
    this.interpolatedVertices = undefined;
    this.lastFrameIndex = -1;
    this.overlay.disconnect();
    this.settleReadiness('off');
  }

  play(startedAt = performance.now()): void {
    this.playFromPhase(0, startedAt);
  }

  playFromPhase(phase: number, now = performance.now()): void {
    const configuration = this.configuration;
    if (!configuration) return;
    this.heldPhase = normalizeSpinnerPhase(phase);
    this.startedAt = now - this.heldPhase * configuration.durationMs;
    this.lastFrameIndex = -1;
    this.syncFrameSubscription();
  }

  setPhase(phase: number): void {
    const configuration = this.configuration;
    if (!configuration) return;
    this.unsubscribeFromFrames();
    const frameIndex = spinnerFrameIndexForPhase(phase, configuration.frameCount);
    this.heldPhase = spinnerPhaseForFrameIndex(frameIndex, configuration.frameCount);
    this.startedAt = null;
    this.renderFrame(frameIndex);
  }

  setFrameIndex(frameIndex: number): void {
    const configuration = this.configuration;
    if (!configuration) return;
    this.setPhase(spinnerPhaseForFrameIndex(frameIndex, configuration.frameCount));
  }

  queueFrameIndex(frameIndex: number): void {
    const configuration = this.configuration;
    if (!configuration) return;
    this.heldPhase = spinnerPhaseForFrameIndex(frameIndex, configuration.frameCount);
    this.renderFrame(frameIndex, true);
  }

  phaseAt(now = performance.now()): number {
    const configuration = this.configuration;
    if (!configuration || this.startedAt === null) return this.heldPhase;
    return normalizeSpinnerPhase((now - this.startedAt) / configuration.durationMs);
  }

  pause(now = performance.now()): void {
    const configuration = this.configuration;
    if (!configuration) return;
    const frameIndex = spinnerFrameIndexForPhase(this.phaseAt(now), configuration.frameCount);
    this.unsubscribeFromFrames();
    this.startedAt = null;
    this.heldPhase = spinnerPhaseForFrameIndex(frameIndex, configuration.frameCount);
    this.renderFrame(frameIndex);
  }

  whenReady(): Promise<PrismSpinnerHdrState> {
    return this.readiness;
  }

  private async load(generation: number): Promise<void> {
    const configuration = this.configuration;
    if (!configuration || this.hdrState !== 'active') return;
    try {
      const asset = await loadAsset(configuration.assetUrl);
      if (generation !== this.generation || configuration !== this.configuration || this.hdrState !== 'active') return;
      if (asset.frames.length !== configuration.frameCount) {
        throw new RangeError(`Spinner HDR asset has ${asset.frames.length} frames; expected ${configuration.frameCount}.`);
      }
      this.asset = asset;
      this.settleReadiness('active');
      if (this.startedAt === null) {
        const frameIndex = spinnerFrameIndexForPhase(this.heldPhase, configuration.frameCount);
        this.renderFrame(frameIndex);
      }
      this.syncFrameSubscription();
    } catch {
      if (generation !== this.generation) return;
      this.unsubscribeFromFrames();
      this.canvas.hidden = true;
      this.onStateChange('failed');
      this.settleReadiness('failed');
    }
  }

  private readonly handleScheduledFrame = (now: number): void => {
    const asset = this.asset;
    const configuration = this.configuration;
    if (!asset || !configuration || this.startedAt === null || this.hdrState !== 'active') return;
    const frameIndex = spinnerFrameIndexForPhase(this.phaseAt(now), configuration.frameCount);
    if (frameIndex !== this.lastFrameIndex) this.renderFrame(frameIndex, true);
  };

  private renderFrame(frameIndex: number, batched = false): void {
    const asset = this.asset;
    const configuration = this.configuration;
    if (!asset || !configuration || this.hdrState !== 'active' || frameIndex === this.lastFrameIndex) return;
    const frame = asset.frames[frameIndex];
    if (!frame) return;
    this.interpolatedVertices = interpolateSpinnerHdrFrame(frame, configuration.headroom, this.interpolatedVertices);
    if (batched) this.overlay.queueRenderVertices(configuration.cssSize, this.interpolatedVertices);
    else this.overlay.renderVertices(configuration.cssSize, this.interpolatedVertices);
    this.lastFrameIndex = frameIndex;
  }

  private settleReadiness(state: PrismSpinnerHdrState): void {
    this.resolveReadiness?.(state);
    this.resolveReadiness = null;
  }

  private syncFrameSubscription(): void {
    const shouldSubscribe = shouldScheduleSpinnerHdrPlayback({
      assetReady: Boolean(this.asset),
      hdrActive: this.hdrState === 'active',
      playing: this.startedAt !== null,
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

export class PrismSpinnerPreRenderedPlayer {
  private atlas: HTMLImageElement | null = null;
  private configuration: PrismSpinnerAtlasConfiguration | null = null;
  private context: CanvasRenderingContext2D;
  private hdrPlayer: PrismSpinnerPreRenderedHdrPlayer | null;
  private playing = false;
  private playback = createSpinnerFramePlayback(2300, 138);
  private readyPromise: Promise<void> = Promise.reject(new Error('Spinner atlas is not connected.'));
  private state: 'failed' | 'loading' | 'ready' = 'loading';
  private unsubscribeFrame: (() => void) | null = null;
  private visible = true;
  private visibilityObserver: IntersectionObserver | null = null;
  private readonly canvas: HTMLCanvasElement;

  constructor(canvas: HTMLCanvasElement, hdrCanvas: HTMLCanvasElement | null = null) {
    this.canvas = canvas;
    const context = canvas.getContext('2d');
    if (!context) throw new Error('A 2D canvas context is unavailable.');
    this.context = context;
    this.hdrPlayer = hdrCanvas ? new PrismSpinnerPreRenderedHdrPlayer(hdrCanvas) : null;
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
      .then(async (atlas) => {
        if (this.configuration !== generationConfiguration) return;
        this.atlas = atlas;
        this.state = 'ready';
        this.setPhase(0);
        if (configuration.hdr && this.hdrPlayer) {
          this.hdrPlayer.connect(configuration.hdr);
          const hdrState = await this.hdrPlayer.whenReady();
          if (this.configuration !== generationConfiguration) return;
          if (hdrState === 'failed' || hdrState === 'unsupported') this.hdrPlayer.disconnect();
        }
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

  setHdrMode(mode: PrismSpinnerHdrMode): void {
    const configuration = this.configuration;
    if (!configuration?.hdr || !this.hdrPlayer) return;
    const nextConfiguration = { ...configuration.hdr, mode };
    configuration.hdr = nextConfiguration;
    this.hdrPlayer.connect(nextConfiguration);
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
    this.hdrPlayer?.disconnect();
    this.hdrPlayer = null;
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
    if (frameIndex !== null) this.drawFrame(frameIndex, true);
  };

  private drawFrame(frameIndex: number, batchedHdr = false): void {
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
    if (batchedHdr) this.hdrPlayer?.queueFrameIndex(frameIndex);
    else this.hdrPlayer?.setFrameIndex(frameIndex);
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
