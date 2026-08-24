import { createPrismIconElement } from './internal/prism-icon-element.ts';
import {
  createPrismIconMorphTrajectory,
  PRISM_ICON_DURATION_SECONDS,
  PRISM_ICON_IDLE_PHASE_TURNS,
  PRISM_ICON_IDLE_RENDER_SCALE,
  samplePrismIconMorphTrajectory,
} from './internal/prism-icon-morph.ts';
import { createPrismModeRoute } from './internal/prism-mode-route.ts';
import { mountPrismObject, PRISM_RENDER_SCALE_REFERENCE_SIZE, prismObjectDefaults } from './internal/prism-object.ts';
import { createPrismPointerInteraction } from './internal/prism-pointer-interaction.ts';
import {
  createFixedPrismSpinnerTrajectory,
  nextForwardSpinnerFrame,
  PRISM_SPINNER_CSS_SIZE,
  resolvePrismSpinnerMorphChannels,
  resolvePrismSpinnerSizeProgress,
  sampleFixedPrismSpinnerTrajectory,
  spinnerPhaseForFrameIndex,
  spinnerPhaseToWorldPhase,
} from './internal/prism-spinner-morph.ts';
import { PrismSpinnerPreRenderedHdrPlayer, PrismSpinnerPreRenderedPlayer } from './internal/prism-spinner-prerendered.ts';
import { PRISM_SPINNER_DURATION_MS, PRISM_SPINNER_FRAME_COUNT, resolvePrismSpinnerAssets } from './spinner-player.ts';
import type { PrismIconMorphSample } from './internal/prism-icon-morph.ts';
import type { PrismModeOwner, PrismModeReadiness } from './internal/prism-mode-route.ts';
import type { PrismWebRenderer } from './internal/prism-wgpu-renderer.ts';

export type PrismTarget = 'icon' | 'prism' | 'spinner';

type PrismEdgeColor = string | readonly [number, number, number, number];

type PrismWebRuntimeInstance = {
  createRenderer(canvas: HTMLCanvasElement, preferHdr: boolean): PrismWebRenderer;
};

type PrismRendererModule = {
  default?: () => Promise<unknown> | unknown;
  PrismWebRuntime: {
    create(canvas: HTMLCanvasElement): Promise<PrismWebRuntimeInstance>;
  };
};

export type PrismRuntimeOptions = {
  loadRenderer: () => Promise<PrismRendererModule>;
};

export type PrismRuntimeObjectOptions = {
  edgeColor?: PrismEdgeColor;
  interactive?: boolean;
  preload?: boolean;
  reducedMotion?: boolean;
  target: PrismTarget;
};

export type PrismRuntimeSpinnerOptions = {
  reducedMotion?: boolean;
};

export type PrismTargetRequestOptions = {
  totalDurationMs?: number;
};

export type PrismRuntimeSnapshot = {
  journeyProgress: number | null;
  owner: PrismModeOwner | 'apng';
  readiness: PrismModeReadiness;
  requestedTarget: PrismTarget;
  settledTarget: PrismTarget | null;
};

export type MountedPrismObject = {
  destroy(): void;
  readonly snapshot: PrismRuntimeSnapshot;
  setTarget(target: PrismTarget, options?: PrismTargetRequestOptions): void;
  subscribe(listener: (snapshot: PrismRuntimeSnapshot) => void): () => void;
  update(options: Pick<PrismRuntimeObjectOptions, 'edgeColor' | 'interactive' | 'reducedMotion'>): void;
  whenReady(): Promise<PrismModeReadiness>;
};

export type MountedPrismSpinner = {
  destroy(): void;
  readonly snapshot: PrismRuntimeSnapshot;
  subscribe(listener: (snapshot: PrismRuntimeSnapshot) => void): () => void;
  update(options: Pick<PrismRuntimeSpinnerOptions, 'reducedMotion'>): void;
};

export type PrismRuntime = {
  mountObject(element: HTMLElement, options: PrismRuntimeObjectOptions): MountedPrismObject;
  mountSpinner(element: HTMLElement, options?: PrismRuntimeSpinnerOptions): MountedPrismSpinner;
};

type PrismObjectController = ReturnType<typeof mountPrismObject>;

const ICON_SIZE = prismObjectDefaults.iconSize;
const CANVAS_SIZE = prismObjectDefaults.canvasSize;
const PRISM_SIZE = prismObjectDefaults.prismSize;
const SPINNER_SIZE = PRISM_SPINNER_CSS_SIZE;
const PRISM_PERIOD_SECONDS = prismObjectDefaults.period;

function clamp(value: number, minimum = 0, maximum = 1): number {
  return Math.max(minimum, Math.min(maximum, value));
}

function interpolatePositive(start: number, target: number, progress: number): number {
  const amount = resolvePrismSpinnerSizeProgress(progress);
  return Math.exp(Math.log(start) * (1 - amount) + Math.log(target) * amount);
}

function requireElement(element: HTMLElement): void {
  if (!(element instanceof HTMLElement)) throw new TypeError('Prism runtime mounts need an HTMLElement.');
}

function requireTarget(target: PrismTarget): void {
  if (target !== 'icon' && target !== 'prism' && target !== 'spinner') {
    throw new RangeError('Unknown Prism target.');
  }
}

function targetDuration(options: PrismTargetRequestOptions | undefined): number | null {
  const duration = options?.totalDurationMs;
  if (duration === undefined) return null;
  if (!Number.isFinite(duration) || duration <= 0) {
    throw new RangeError('Prism target journey duration must be finite and greater than zero.');
  }
  return duration;
}

function normalizeEdgeColor(value: PrismEdgeColor | undefined, element: HTMLElement): [number, number, number, number] {
  if (Array.isArray(value)) {
    if (value.length !== 4 || value.some((component) => !(Number.isFinite(component) && component >= 0 && component <= 1))) {
      throw new RangeError('Prism edge color must contain four normalized components.');
    }
    return [...value] as [number, number, number, number];
  }

  const document = element.ownerDocument ?? globalThis.document;
  const canvas = document.createElement('canvas');
  canvas.width = 1;
  canvas.height = 1;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  if (!context) return [0.5, 0.5, 0.5, 1];
  context.clearRect(0, 0, 1, 1);
  context.fillStyle = typeof value === 'string' ? value : getComputedStyle(element).color;
  context.fillRect(0, 0, 1, 1);
  const [red = 128, green = 128, blue = 128, alpha = 255] = context.getImageData(0, 0, 1, 1).data;
  return [red / 255, green / 255, blue / 255, alpha / 255];
}

function setOwnerVisibility(svg: SVGSVGElement, canvas: HTMLCanvasElement, atlas: HTMLElement, owner: PrismModeOwner): void {
  svg.style.setProperty('display', owner === 'svg' ? 'block' : 'none');
  canvas.style.setProperty('display', owner === 'webgpu' ? 'block' : 'none');
  atlas.style.setProperty('display', owner === 'atlas' ? 'grid' : 'none');
}

export function createPrismRuntime(options: PrismRuntimeOptions): PrismRuntime {
  if (typeof options?.loadRenderer !== 'function') {
    throw new TypeError('createPrismRuntime needs a renderer loader.');
  }

  let modulePromise: Promise<PrismRendererModule> | null = null;
  let rendererRuntimePromise: Promise<PrismWebRuntimeInstance> | null = null;

  function loadModule(): Promise<PrismRendererModule> {
    modulePromise ??= Promise.try(options.loadRenderer);
    return modulePromise;
  }

  function loadRuntime(canvas: HTMLCanvasElement): Promise<PrismWebRuntimeInstance> {
    rendererRuntimePromise ??= loadModule().then(async (module) => {
      await module.default?.();
      return module.PrismWebRuntime.create(canvas);
    });
    return rendererRuntimePromise;
  }

  async function createRenderer(canvas: HTMLCanvasElement, preferHdr: boolean): Promise<PrismWebRenderer> {
    const runtime = await loadRuntime(canvas);
    return runtime.createRenderer(canvas, preferHdr);
  }

  function mountObject(element: HTMLElement, initialOptions: PrismRuntimeObjectOptions): MountedPrismObject {
    requireElement(element);
    requireTarget(initialOptions.target);
    const document = element.ownerDocument ?? globalThis.document;
    const root = document.createElement('span');
    const svg = createPrismIconElement(document, ICON_SIZE);
    const canvas = document.createElement('canvas');
    const atlas = document.createElement('span');
    const atlasCanvas = document.createElement('canvas');
    const atlasHdrCanvas = document.createElement('canvas');
    root.className = 'prism-object';
    root.style.setProperty('display', 'grid');
    root.style.setProperty('height', `${CANVAS_SIZE}px`);
    root.style.setProperty('place-items', 'center');
    root.style.setProperty('position', 'relative');
    root.style.setProperty('width', `${CANVAS_SIZE}px`);
    svg.style.setProperty('grid-area', '1 / 1');
    canvas.style.setProperty('grid-area', '1 / 1');
    canvas.style.setProperty('height', `${CANVAS_SIZE}px`);
    canvas.style.setProperty('width', `${CANVAS_SIZE}px`);
    atlas.style.setProperty('grid-area', '1 / 1');
    atlas.style.setProperty('height', `${SPINNER_SIZE}px`);
    atlas.style.setProperty('place-items', 'center');
    atlas.style.setProperty('width', `${SPINNER_SIZE}px`);
    atlasCanvas.style.setProperty('grid-area', '1 / 1');
    atlasCanvas.style.setProperty('height', `${SPINNER_SIZE}px`);
    atlasCanvas.style.setProperty('width', `${SPINNER_SIZE}px`);
    atlasHdrCanvas.style.setProperty('grid-area', '1 / 1');
    atlasHdrCanvas.style.setProperty('height', `${SPINNER_SIZE}px`);
    atlasHdrCanvas.style.setProperty('width', `${SPINNER_SIZE}px`);
    atlas.append(atlasCanvas, atlasHdrCanvas);
    root.append(svg, canvas, atlas);
    element.replaceChildren(root);

    let edgeColor = normalizeEdgeColor(initialOptions.edgeColor, element);
    let reducedMotion = Boolean(initialOptions.reducedMotion);
    let interactive = Boolean(initialOptions.interactive);
    let destroyed = false;
    let controller: PrismObjectController | null = null;
    let controllerPromise: Promise<PrismObjectController | null> | null = null;
    let controllerUnavailableUnsubscribe: (() => void) | null = null;
    let frameId = 0;
    let interactionFrameId = 0;
    let pointerKinematicsCurrent = false;
    let presentationFrameId = 0;
    let warmupFrameId = 0;
    let resolveWarmup: (() => void) | null = null;
    let transitionGeneration = 0;
    let readiness: PrismModeReadiness = 'loading';
    let atlasReadiness: PrismModeReadiness = 'loading';
    let atlasPromise: Promise<void> | null = null;
    let atlasPreloadIdleId = 0;
    let atlasPreloadTimer: ReturnType<typeof setTimeout> | null = null;
    let atlasWorldPhase = PRISM_ICON_IDLE_PHASE_TURNS;
    let atlasStartedAt = 0;
    let requestedTarget = initialOptions.target;
    let settledTarget: PrismTarget | null = 'icon';
    let journeyProgress: number | null = null;
    let journey: {
      completedDistance: number;
      endSizeScale: number;
      startedAt: number | null;
      startSizeScale: number;
      totalDistance: number;
      totalDurationMs: number;
    } | null = null;
    let currentSizeScale = PRISM_ICON_IDLE_RENDER_SCALE;
    let sizeScaleOverride: number | null = null;
    const listeners = new Set<(snapshot: PrismRuntimeSnapshot) => void>();
    const route = createPrismModeRoute({ reducedMotion });
    route.syncKinematics({ phase: PRISM_ICON_IDLE_PHASE_TURNS, velocity: 0 });
    const atlasPlayer = new PrismSpinnerPreRenderedPlayer(atlasCanvas, atlasHdrCanvas);
    const pointerInteraction = createPrismPointerInteraction(element, {
      lightPeriod: prismObjectDefaults.lightPeriod,
      period: PRISM_PERIOD_SECONDS,
      prismSize: PRISM_SIZE,
    });

    function snapshot(): PrismRuntimeSnapshot {
      return {
        journeyProgress,
        owner: route.snapshot.owner,
        readiness,
        requestedTarget,
        settledTarget,
      };
    }

    function notify(): void {
      const next = snapshot();
      for (const listener of listeners) listener(next);
    }

    function pointerInteractionAllowed(): boolean {
      return (
        interactive &&
        !reducedMotion &&
        readiness === 'ready' &&
        requestedTarget === 'prism' &&
        settledTarget === 'prism' &&
        route.snapshot.owner === 'webgpu' &&
        frameId === 0
      );
    }

    function resetPointerInteractionAppearance(): void {
      controller?.update({
        dispersion: prismObjectDefaults.dispersion,
        hdrHeadroom: prismObjectDefaults.hdrHeadroom,
        lightPhaseOffset: prismObjectDefaults.lightPhaseOffset,
        sheenStrength: prismObjectDefaults.sheenStrength,
        sourceHalo: prismObjectDefaults.sourceHalo,
      });
    }

    function stopPointerInteraction(): void {
      pointerInteraction.setInputEnabled(false);
      if (interactionFrameId !== 0) {
        cancelAnimationFrame(interactionFrameId);
        interactionFrameId = 0;
      }
    }

    function staticPresentationRequired(): boolean {
      return reducedMotion || readiness === 'unavailable';
    }

    function settleStaticPresentation(): void {
      transitionGeneration += 1;
      if (frameId) cancelAnimationFrame(frameId);
      if (presentationFrameId) cancelAnimationFrame(presentationFrameId);
      if (warmupFrameId) cancelAnimationFrame(warmupFrameId);
      frameId = 0;
      presentationFrameId = 0;
      warmupFrameId = 0;
      resolveWarmup?.();
      resolveWarmup = null;
      stopPointerInteraction();
      atlasPlayer.pause();
      controller?.setActive(false);
      controller?.setIconMorphSample(null);
      controller?.setMorphPoseQuaternion(null);
      controller?.setRotationPhase(PRISM_ICON_IDLE_PHASE_TURNS);
      controller?.update({ sizeScaleOverride: null, spinnerMorphProgress: 0 });
      route.syncKinematics({ phase: PRISM_ICON_IDLE_PHASE_TURNS, velocity: 0 });
      route.syncState({ iconProgress: 0, owner: 'svg', spinnerProgress: 0 });
      currentSizeScale = PRISM_ICON_IDLE_RENDER_SCALE;
      journey = null;
      journeyProgress = null;
      settledTarget = 'icon';
      sizeScaleOverride = null;
      setOwnerVisibility(svg, canvas, atlas, 'svg');
      notify();
    }

    function handleRendererUnavailable(): void {
      if (destroyed || readiness === 'unavailable') return;
      readiness = 'unavailable';
      route.setRendererReadiness('unavailable');
      settleStaticPresentation();
    }

    function syncPointerInteraction(): void {
      if (!controller || !pointerInteractionAllowed()) {
        stopPointerInteraction();
        return;
      }
      pointerInteraction.setInputEnabled(true);
      if (interactionFrameId !== 0) return;
      pointerInteraction.reset(controller.poseSnapshot);
      const step = (now: number): void => {
        interactionFrameId = 0;
        if (!controller || !pointerInteractionAllowed()) {
          stopPointerInteraction();
          return;
        }
        const sample = pointerInteraction.sample(now);
        currentSizeScale = (PRISM_SIZE / PRISM_RENDER_SCALE_REFERENCE_SIZE) * sample.scale;
        sizeScaleOverride = currentSizeScale;
        controller.setRotationPhase(sample.phase);
        controller.setMorphPoseQuaternion(sample.orientation);
        controller.update({
          dispersion: sample.dispersion,
          hdrHeadroom: sample.hdrHeadroom,
          lightPhaseOffset: sample.lightPhaseOffset,
          sheenStrength: sample.sheenStrength,
          sizeScaleOverride,
          sourceHalo: sample.sourceHalo,
        });
        route.syncKinematics({
          angularVelocity: sample.angularVelocity,
          orientation: sample.orientation,
          phase: sample.phase,
          velocity: sample.velocity,
        });
        pointerKinematicsCurrent = true;
        interactionFrameId = requestAnimationFrame(step);
      };
      interactionFrameId = requestAnimationFrame(step);
    }

    function presentOwner(owner: PrismModeOwner): void {
      if (presentationFrameId !== 0 && owner === 'webgpu') return;
      setOwnerVisibility(svg, canvas, atlas, owner);
    }

    function handOffSvgToWebGpuAfterPaint(): void {
      if (presentationFrameId !== 0) return;
      svg.style.setProperty('display', 'block');
      canvas.style.setProperty('display', 'block');
      canvas.style.setProperty('visibility', 'hidden');
      atlas.style.setProperty('display', 'none');
      presentationFrameId = requestAnimationFrame(() => {
        if (destroyed) {
          presentationFrameId = 0;
          return;
        }
        canvas.style.removeProperty('visibility');
        presentationFrameId = requestAnimationFrame(() => {
          presentationFrameId = 0;
          if (!destroyed) setOwnerVisibility(svg, canvas, atlas, route.snapshot.owner);
        });
      });
    }

    function remainingDistance(target: PrismTarget): number {
      const state = route.snapshot;
      if (target === 'spinner') return 1 - state.iconProgress + (1 - state.spinnerProgress);
      if (target === 'prism') return 1 - state.iconProgress + state.spinnerProgress;
      return state.iconProgress + state.spinnerProgress;
    }

    function actionDistance(action: NonNullable<ReturnType<typeof route.nextAction>>): number {
      return Math.abs(action.targetProgress - action.startProgress);
    }

    function actionDurationSeconds(action: NonNullable<ReturnType<typeof route.nextAction>>, defaultDurationSeconds: number): number {
      if (!journey) return defaultDurationSeconds;
      return (journey.totalDurationMs * actionDistance(action)) / journey.totalDistance / 1000;
    }

    function targetSizeScale(target: PrismTarget): number {
      if (target === 'icon') return PRISM_ICON_IDLE_RENDER_SCALE;
      if (target === 'spinner') return SPINNER_SIZE / PRISM_RENDER_SCALE_REFERENCE_SIZE;
      return PRISM_SIZE / PRISM_RENDER_SCALE_REFERENCE_SIZE;
    }

    function updateJourneySizeScale(): void {
      if (!journey || journeyProgress === null) return;
      currentSizeScale = interpolatePositive(journey.startSizeScale, journey.endSizeScale, journeyProgress);
      sizeScaleOverride = currentSizeScale;
      controller?.update({ sizeScaleOverride });
    }

    function actionProgress(
      action: NonNullable<ReturnType<typeof route.nextAction>>,
      now: number,
      localStartedAt: { value: number | null },
      durationMs: number,
    ): number {
      if (!journey) {
        localStartedAt.value ??= now;
        return clamp((now - localStartedAt.value) / durationMs);
      }
      journey.startedAt ??= now;
      const segmentStartedAt = journey.startedAt + (journey.totalDurationMs * journey.completedDistance) / journey.totalDistance;
      const progress = clamp((now - segmentStartedAt) / durationMs);
      journeyProgress = clamp((journey.completedDistance + actionDistance(action) * progress) / journey.totalDistance);
      updateJourneySizeScale();
      return progress;
    }

    function completeJourneyAction(action: NonNullable<ReturnType<typeof route.nextAction>>): void {
      if (!journey) return;
      journey.completedDistance = Math.min(journey.completedDistance + actionDistance(action), journey.totalDistance);
      journeyProgress = clamp(journey.completedDistance / journey.totalDistance);
      updateJourneySizeScale();
    }

    function finishJourney(): void {
      if (!journey) return;
      journeyProgress = 1;
      updateJourneySizeScale();
      journey = null;
      sizeScaleOverride = null;
      controller?.update({ sizeScaleOverride: null });
    }

    function currentAtlasWorldPhase(now = performance.now()): number {
      const frameIndex = atlasPlayer.frameIndexAt(now);
      const framePhase = spinnerPhaseForFrameIndex(frameIndex, PRISM_SPINNER_FRAME_COUNT);
      const unquantized = atlasWorldPhase + Math.max(now - atlasStartedAt, 0) / PRISM_SPINNER_DURATION_MS;
      const phaseWithinTurn = spinnerPhaseToWorldPhase(framePhase, 0);
      return phaseWithinTurn + Math.round(unquantized - phaseWithinTurn);
    }

    function handOffWebGpuToAtlas(now = performance.now()): void {
      if (!controller || atlasReadiness !== 'ready' || route.snapshot.owner !== 'webgpu') return;
      const handoff = nextForwardSpinnerFrame(controller.rotationPhase, 0, PRISM_SPINNER_FRAME_COUNT);
      controller.setRotationPhase(handoff.worldPhase);
      controller.setMorphPoseQuaternion(null);
      controller.setActive(false);
      atlasPlayer.playFromPhase(handoff.framePhase, now);
      atlasWorldPhase = handoff.worldPhase;
      atlasStartedAt = now;
      route.syncKinematics({
        phase: handoff.worldPhase,
        velocity: 1000 / PRISM_SPINNER_DURATION_MS,
      });
      route.syncState({ owner: 'atlas', spinnerProgress: 1 });
      setOwnerVisibility(svg, canvas, atlas, 'atlas');
      notify();
    }

    function handOffAtlasToWebGpu(now = performance.now()): void {
      if (!controller || route.snapshot.owner !== 'atlas') return;
      const phase = currentAtlasWorldPhase(now);
      atlasPlayer.pause(now);
      controller.update({
        period: PRISM_SPINNER_DURATION_MS / 1000,
        spinnerMorphProgress: 1,
      });
      controller.setMorphPoseQuaternion(null);
      controller.setRotationPhase(phase);
      controller.setActive(true);
      route.syncKinematics({
        phase,
        velocity: 1000 / PRISM_SPINNER_DURATION_MS,
      });
      route.syncState({ owner: 'webgpu', spinnerProgress: 1 });
      setOwnerVisibility(svg, canvas, atlas, 'webgpu');
    }

    function cancelAtlasPreload(): void {
      if (atlasPreloadIdleId !== 0) {
        cancelIdleCallback(atlasPreloadIdleId);
        atlasPreloadIdleId = 0;
      }
      if (atlasPreloadTimer !== null) {
        clearTimeout(atlasPreloadTimer);
        atlasPreloadTimer = null;
      }
    }

    function scheduleAtlasPreload(): void {
      if (destroyed || atlasPromise || atlasPreloadIdleId !== 0 || atlasPreloadTimer !== null) return;
      if (typeof requestIdleCallback === 'function') {
        atlasPreloadIdleId = requestIdleCallback(() => {
          atlasPreloadIdleId = 0;
          void ensureAtlas();
        });
      } else {
        atlasPreloadTimer = setTimeout(() => {
          atlasPreloadTimer = null;
          void ensureAtlas();
        }, 0);
      }
    }

    function ensureAtlas(): Promise<void> {
      cancelAtlasPreload();
      if (atlasPromise) return atlasPromise;
      atlasPromise = atlasPlayer
        .connect(resolvePrismSpinnerAssets(devicePixelRatio).atlas)
        .then(() => {
          if (destroyed) return;
          atlasReadiness = 'ready';
          route.setSpinnerReadiness('ready');
          if (requestedTarget === 'spinner' && route.snapshot.spinnerProgress >= 1 - 0.0001) {
            handOffWebGpuToAtlas();
            settledTarget = 'spinner';
            finishJourney();
          }
          notify();
          settleRoute();
        })
        .catch(() => {
          if (destroyed) return;
          atlasReadiness = 'unavailable';
          route.setSpinnerReadiness(readiness);
          notify();
          settleRoute();
        });
      return atlasPromise;
    }

    function ensureController(): Promise<PrismObjectController | null> {
      if (controllerPromise) return controllerPromise;
      controller = mountPrismObject(canvas, {
        canvasSize: CANVAS_SIZE,
        createRenderer,
        iconEdgeColor: edgeColor,
        iconSize: ICON_SIZE,
        period: PRISM_PERIOD_SECONDS,
        prismSize: PRISM_SIZE,
        spinnerSize: SPINNER_SIZE,
        transitionProgress: route.snapshot.iconProgress,
        sizeScaleOverride,
      });
      controllerUnavailableUnsubscribe = controller.onUnavailable(handleRendererUnavailable);
      const idleSample = samplePrismIconMorphTrajectory(
        createPrismIconMorphTrajectory({
          durationSeconds: PRISM_ICON_DURATION_SECONDS,
          iconSize: ICON_SIZE,
          prismSize: PRISM_SIZE,
          prismVelocity: 1 / PRISM_PERIOD_SECONDS,
          startPhase: PRISM_ICON_IDLE_PHASE_TURNS,
          startProgress: 0,
          startVelocity: 0,
          targetProgress: 1,
        }),
        0,
      );
      controller.setIconMorphSample(idleSample);
      controller.setRotationPhase(idleSample.phase);
      controller.setActive(false);
      controllerPromise = controller.whenReady().then(async (state) => {
        if (destroyed) return null;
        if (state === 'ready') {
          canvas.style.setProperty('display', 'block');
          canvas.style.setProperty('visibility', 'hidden');
          controller?.setActive(true);
          await new Promise<void>((resolve) => {
            let remaining = 2;
            resolveWarmup = resolve;
            const step = () => {
              warmupFrameId = 0;
              if (destroyed || --remaining === 0) {
                resolveWarmup = null;
                resolve();
                return;
              }
              warmupFrameId = requestAnimationFrame(step);
            };
            warmupFrameId = requestAnimationFrame(step);
          });
          if (destroyed) return null;
          controller?.setActive(false);
          canvas.style.removeProperty('visibility');
          setOwnerVisibility(svg, canvas, atlas, route.snapshot.owner);
        }
        readiness = state === 'ready' ? 'ready' : 'unavailable';
        route.setRendererReadiness(readiness);
        route.setSpinnerReadiness(atlasReadiness === 'loading' ? 'loading' : readiness);
        if (readiness === 'ready' && initialOptions.preload) scheduleAtlasPreload();
        if (readiness === 'ready') settleRoute();
        else setOwnerVisibility(svg, canvas, atlas, 'svg');
        notify();
        return controller;
      });
      return controllerPromise;
    }

    function renderIconSample(sample: PrismIconMorphSample): void {
      if (!controller || destroyed) return;
      if (sizeScaleOverride === null) currentSizeScale = sample.renderScale;
      const previousOwner = route.snapshot.owner;
      controller.setIconMorphSample(sample);
      controller.setRotationPhase(sample.phase);
      route.recordIconSample(sample);
      if (previousOwner === 'svg' && route.snapshot.owner === 'webgpu') handOffSvgToWebGpuAfterPaint();
      else presentOwner(route.snapshot.owner);
      notify();
    }

    function finishIconTransition(sample: PrismIconMorphSample, targetProgress: 0 | 1, now?: number): void {
      if (!controller || destroyed) return;
      route.completeIcon(sample);
      controller.setIconMorphSample(null);
      if (targetProgress === 0) {
        controller.setActive(false);
        settledTarget = 'icon';
      } else {
        controller.setRotationPhase(sample.phase);
        controller.resumeRotation();
        settledTarget = requestedTarget === 'prism' ? 'prism' : null;
      }
      presentOwner(route.snapshot.owner);
      notify();
      if (settledTarget === 'prism') scheduleAtlasPreload();
      settleRoute(now);
    }

    function animateIconTransition(action: NonNullable<ReturnType<typeof route.nextAction>>, inheritedNow?: number): void {
      if (!controller || action.kind !== 'icon') return;
      pointerKinematicsCurrent = false;
      route.begin(action);
      const generation = ++transitionGeneration;
      const distance = Math.max(Math.abs(action.targetProgress - action.startProgress), 0.0001);
      const durationSeconds = actionDurationSeconds(action, PRISM_ICON_DURATION_SECONDS * distance);
      const trajectory = createPrismIconMorphTrajectory({
        durationSeconds,
        iconSize: ICON_SIZE,
        prismSize: PRISM_SIZE,
        prismVelocity: 1 / PRISM_PERIOD_SECONDS,
        startAngularVelocity: action.startAngularVelocity,
        startEdgeHighlightProgress: action.startEdgeHighlightProgress,
        startOrientation: action.startOrientation,
        startPhase: action.startPhase,
        startProgress: action.startProgress,
        startVelocity: action.startVelocity,
        targetProgress: action.targetProgress,
      });
      const first = samplePrismIconMorphTrajectory(trajectory, 0);
      const last = samplePrismIconMorphTrajectory(trajectory, 1);
      controller.setActive(true);
      renderIconSample(first);

      if (action.instant) {
        completeJourneyAction(action);
        renderIconSample(last);
        finishIconTransition(last, action.targetProgress, inheritedNow);
        return;
      }

      const startedAt = { value: null as number | null };
      const durationMs = Math.max(trajectory.durationSeconds * 1000, 1);
      const step = (now: number): void => {
        if (destroyed || generation !== transitionGeneration) return;
        const progress = actionProgress(action, now, startedAt, durationMs);
        const sample = samplePrismIconMorphTrajectory(trajectory, progress);
        renderIconSample(sample);
        if (progress < 1) frameId = requestAnimationFrame(step);
        else {
          frameId = 0;
          completeJourneyAction(action);
          finishIconTransition(last, action.targetProgress, now);
        }
      };
      if (inheritedNow === undefined) frameId = requestAnimationFrame(step);
      else step(inheritedNow);
    }

    function renderSpinnerSample(sample: ReturnType<typeof sampleFixedPrismSpinnerTrajectory>): void {
      if (!controller || destroyed) return;
      if (sizeScaleOverride === null) {
        currentSizeScale =
          resolvePrismSpinnerMorphChannels(sample.progress, PRISM_SIZE, SPINNER_SIZE).size / PRISM_RENDER_SCALE_REFERENCE_SIZE;
      }
      controller.setMorphPoseQuaternion(sample.orientation);
      controller.setRotationPhase(sample.phase);
      controller.update({ spinnerMorphProgress: sample.progress });
      route.recordSpinnerSample(sample);
      setOwnerVisibility(svg, canvas, atlas, 'webgpu');
      notify();
    }

    function finishSpinnerTransition(
      sample: ReturnType<typeof sampleFixedPrismSpinnerTrajectory>,
      targetProgress: 0 | 1,
      handoff: ReturnType<typeof nextForwardSpinnerFrame> | null,
      now?: number,
    ): void {
      if (!controller || destroyed) return;
      if (targetProgress === 1 && handoff && atlasReadiness === 'ready') {
        controller.setRotationPhase(handoff.worldPhase);
        atlasPlayer.playFromPhase(handoff.framePhase);
        atlasWorldPhase = handoff.worldPhase;
        atlasStartedAt = performance.now();
        controller.setMorphPoseQuaternion(null);
        controller.setActive(false);
        route.completeSpinner(sample, 'atlas');
        settledTarget = 'spinner';
      } else if (targetProgress === 1) {
        controller.update({
          period: PRISM_SPINNER_DURATION_MS / 1000,
          spinnerMorphProgress: 1,
        });
        controller.setMorphPoseQuaternion(null);
        controller.setRotationPhase(sample.phase);
        controller.resumeRotation();
        route.completeSpinner(sample, 'webgpu');
        settledTarget = 'spinner';
      } else {
        controller.update({
          period: PRISM_PERIOD_SECONDS,
          spinnerMorphProgress: 0,
        });
        controller.setMorphPoseQuaternion(null);
        controller.setRotationPhase(sample.phase);
        controller.resumeRotation();
        route.completeSpinner(sample, 'webgpu');
        settledTarget = requestedTarget === 'prism' ? 'prism' : null;
      }
      setOwnerVisibility(svg, canvas, atlas, route.snapshot.owner);
      notify();
      settleRoute(now);
    }

    function animateSpinnerTransition(action: NonNullable<ReturnType<typeof route.nextAction>>, inheritedNow?: number): void {
      if (!controller || action.kind !== 'spinner') return;
      let currentAction = action;
      if (!pointerKinematicsCurrent && route.snapshot.owner === 'webgpu' && currentAction.startProgress <= 0.0001) {
        route.syncKinematics(controller.poseSnapshot);
        const refreshedAction = route.nextAction();
        if (!refreshedAction || refreshedAction.kind !== 'spinner') return;
        currentAction = refreshedAction;
      }
      pointerKinematicsCurrent = false;
      route.begin(currentAction);
      const generation = ++transitionGeneration;
      const distance = Math.max(Math.abs(currentAction.targetProgress - currentAction.startProgress), 0.0001);
      const durationSeconds = actionDurationSeconds(currentAction, PRISM_ICON_DURATION_SECONDS * distance);
      const trajectory = createFixedPrismSpinnerTrajectory({
        frameCount: PRISM_SPINNER_FRAME_COUNT,
        prismVelocity: 1 / PRISM_PERIOD_SECONDS,
        spinnerVelocity: 1000 / PRISM_SPINNER_DURATION_MS,
        startAngularVelocity: currentAction.startAngularVelocity,
        startOrientation: currentAction.startOrientation,
        startPhase: currentAction.startPhase,
        startProgress: currentAction.startProgress,
        startVelocity: currentAction.startVelocity || 1 / PRISM_PERIOD_SECONDS,
        targetProgress: currentAction.targetProgress,
        totalDurationSeconds: durationSeconds,
      });
      const first = sampleFixedPrismSpinnerTrajectory(trajectory, 0);
      const last = sampleFixedPrismSpinnerTrajectory(trajectory, 1);
      controller.setActive(true);
      controller.update({ spinnerMorphMatchPhase: trajectory.matchPhase });
      renderSpinnerSample(first);

      if (currentAction.instant) {
        completeJourneyAction(currentAction);
        renderSpinnerSample(last);
        finishSpinnerTransition(last, currentAction.targetProgress, trajectory.handoff, inheritedNow);
        return;
      }

      const startedAt = { value: null as number | null };
      const durationMs = Math.max(trajectory.durationSeconds * 1000, 1);
      const step = (now: number): void => {
        if (destroyed || generation !== transitionGeneration) return;
        const progress = actionProgress(currentAction, now, startedAt, durationMs);
        const sample = sampleFixedPrismSpinnerTrajectory(trajectory, progress);
        renderSpinnerSample(sample);
        if (progress < 1) frameId = requestAnimationFrame(step);
        else {
          frameId = 0;
          completeJourneyAction(currentAction);
          finishSpinnerTransition(last, currentAction.targetProgress, trajectory.handoff, now);
        }
      };
      if (inheritedNow === undefined) frameId = requestAnimationFrame(step);
      else step(inheritedNow);
    }

    function settleRoute(now?: number): void {
      if (destroyed || frameId || readiness !== 'ready' || staticPresentationRequired()) return;
      const state = route.snapshot;
      if (requestedTarget === 'spinner' && state.spinnerProgress >= 1 - 0.0001) {
        stopPointerInteraction();
        if (atlasReadiness === 'ready' && state.owner === 'webgpu') handOffWebGpuToAtlas();
        settledTarget = 'spinner';
        finishJourney();
        return;
      }
      if (requestedTarget !== 'spinner' && state.owner === 'atlas') handOffAtlasToWebGpu();
      const action = route.nextAction();
      if (!action) {
        if (requestedTarget === 'prism' && route.snapshot.iconProgress >= 1 - 0.0001) settledTarget = 'prism';
        if (settledTarget === requestedTarget) finishJourney();
        syncPointerInteraction();
        return;
      }
      if (action.kind === 'icon') animateIconTransition(action, now);
      else animateSpinnerTransition(action, now);
    }

    function setTarget(target: PrismTarget, requestOptions?: PrismTargetRequestOptions): void {
      requireTarget(target);
      const durationMs = targetDuration(requestOptions);
      if (destroyed || target === requestedTarget) return;
      stopPointerInteraction();
      resetPointerInteractionAppearance();
      requestedTarget = target;
      settledTarget = null;
      if (durationMs === null) {
        journey = null;
        journeyProgress = null;
        sizeScaleOverride = null;
        controller?.update({ sizeScaleOverride: null });
      } else {
        journey = {
          completedDistance: 0,
          endSizeScale: targetSizeScale(target),
          startedAt: null,
          startSizeScale: currentSizeScale,
          totalDistance: Math.max(remainingDistance(target), 0.0001),
          totalDurationMs: durationMs,
        };
        journeyProgress = 0;
        updateJourneySizeScale();
      }
      route.request(target);
      transitionGeneration += 1;
      if (frameId) cancelAnimationFrame(frameId);
      frameId = 0;
      if (staticPresentationRequired()) {
        settleStaticPresentation();
        return;
      }
      notify();
      if (target === 'spinner') void ensureAtlas();
      if (target !== 'icon' || controller) void ensureController().then(() => settleRoute());
    }

    setOwnerVisibility(svg, canvas, atlas, 'svg');
    notify();
    if (initialOptions.target !== 'icon') {
      requestedTarget = 'icon';
      setTarget(initialOptions.target);
    }

    return {
      destroy() {
        if (destroyed) return;
        destroyed = true;
        transitionGeneration += 1;
        if (frameId) cancelAnimationFrame(frameId);
        if (presentationFrameId) cancelAnimationFrame(presentationFrameId);
        if (warmupFrameId) cancelAnimationFrame(warmupFrameId);
        if (interactionFrameId) cancelAnimationFrame(interactionFrameId);
        cancelAtlasPreload();
        resolveWarmup?.();
        resolveWarmup = null;
        atlasPlayer.dispose();
        pointerInteraction.destroy();
        controllerUnavailableUnsubscribe?.();
        controllerUnavailableUnsubscribe = null;
        controller?.dispose();
        listeners.clear();
        root.remove();
      },
      get snapshot() {
        return snapshot();
      },
      setTarget,
      subscribe(listener) {
        listeners.add(listener);
        listener(snapshot());
        return () => listeners.delete(listener);
      },
      update(next) {
        const wasReducedMotion = reducedMotion;
        if (next.edgeColor !== undefined) {
          edgeColor = normalizeEdgeColor(next.edgeColor, element);
          controller?.update({ iconEdgeColor: edgeColor });
        }
        if (next.reducedMotion !== undefined) {
          reducedMotion = Boolean(next.reducedMotion);
          route.setReducedMotion(reducedMotion);
        }
        if (next.interactive !== undefined) interactive = Boolean(next.interactive);
        if (staticPresentationRequired()) {
          if (!wasReducedMotion && reducedMotion) settleStaticPresentation();
          else stopPointerInteraction();
          return;
        }
        if (wasReducedMotion) void ensureController().then(() => settleRoute());
        syncPointerInteraction();
      },
      whenReady() {
        if (readiness !== 'loading') return Promise.resolve(readiness);
        return ensureController().then(() => readiness);
      },
    };
  }

  function mountSpinner(element: HTMLElement, spinnerOptions: PrismRuntimeSpinnerOptions = {}): MountedPrismSpinner {
    requireElement(element);
    const document = element.ownerDocument ?? globalThis.document;
    const root = document.createElement('span');
    const image = document.createElement('img');
    const canvas = document.createElement('canvas');
    const hdrCanvas = document.createElement('canvas');
    root.className = 'prism-spinner';
    root.style.setProperty('display', 'grid');
    root.style.setProperty('height', `${SPINNER_SIZE}px`);
    root.style.setProperty('place-items', 'center');
    root.style.setProperty('width', `${SPINNER_SIZE}px`);
    image.setAttribute('aria-hidden', 'true');
    image.decoding = 'async';
    image.style.setProperty('grid-area', '1 / 1');
    image.style.setProperty('height', `${SPINNER_SIZE}px`);
    image.style.setProperty('width', `${SPINNER_SIZE}px`);
    canvas.style.setProperty('grid-area', '1 / 1');
    canvas.style.setProperty('height', `${SPINNER_SIZE}px`);
    canvas.style.setProperty('width', `${SPINNER_SIZE}px`);
    hdrCanvas.style.setProperty('grid-area', '1 / 1');
    hdrCanvas.style.setProperty('height', `${SPINNER_SIZE}px`);
    hdrCanvas.style.setProperty('width', `${SPINNER_SIZE}px`);
    root.append(image, canvas, hdrCanvas);
    element.replaceChildren(root);

    const listeners = new Set<(snapshot: PrismRuntimeSnapshot) => void>();
    let destroyed = false;
    let reducedMotion = Boolean(spinnerOptions.reducedMotion);
    let apngLoadGeneration = 0;
    const apngHdrPlayer = new PrismSpinnerPreRenderedHdrPlayer(hdrCanvas);
    let atlasPlayer: PrismSpinnerPreRenderedPlayer | null = null;
    let atlasPromise: Promise<void> | null = null;
    const snapshot: PrismRuntimeSnapshot = {
      journeyProgress: null,
      owner: 'apng',
      readiness: 'loading',
      requestedTarget: 'spinner',
      settledTarget: null,
    };

    function notify(): void {
      const next = { ...snapshot };
      for (const listener of listeners) listener(next);
    }

    function show(owner: 'apng' | 'atlas'): void {
      image.style.setProperty('display', owner === 'apng' ? 'block' : 'none');
      canvas.style.setProperty('display', owner === 'atlas' ? 'block' : 'none');
      hdrCanvas.style.setProperty('display', 'block');
      snapshot.owner = owner;
    }

    function ensureAtlas(): Promise<void> {
      if (atlasPromise) return atlasPromise;
      apngLoadGeneration += 1;
      apngHdrPlayer.disconnect();
      const player = new PrismSpinnerPreRenderedPlayer(canvas, hdrCanvas);
      atlasPlayer = player;
      atlasPromise = player
        .connect(resolvePrismSpinnerAssets(devicePixelRatio).atlas)
        .then(() => {
          if (destroyed || atlasPlayer !== player) return;
          if (reducedMotion) player.setPhase(0);
          else player.playFromPhase(0);
          show('atlas');
          snapshot.readiness = 'ready';
          snapshot.settledTarget = 'spinner';
          notify();
        })
        .catch(() => {
          if (destroyed || atlasPlayer !== player) return;
          snapshot.readiness = 'unavailable';
          snapshot.settledTarget = null;
          notify();
        });
      return atlasPromise;
    }

    async function loadApng(): Promise<void> {
      const assets = resolvePrismSpinnerAssets(devicePixelRatio);
      const generation = ++apngLoadGeneration;
      atlasPlayer?.dispose();
      atlasPlayer = null;
      atlasPromise = null;
      apngHdrPlayer.connect(assets.hdr);
      const source = new URL(assets.apngUrl);
      source.searchParams.set('animation-run', String(generation));
      const loadedAt = new Promise<number>((resolve, reject) => {
        const cleanup = () => {
          image.removeEventListener('load', onLoad);
          image.removeEventListener('error', onError);
        };
        const onLoad = () => {
          cleanup();
          resolve(performance.now());
        };
        const onError = () => {
          cleanup();
          reject(new Error('Unable to load the Prism spinner APNG.'));
        };
        image.addEventListener('load', onLoad);
        image.addEventListener('error', onError);
      });
      image.src = source.href;
      try {
        const startedAt = await loadedAt;
        await image.decode();
        if (destroyed || generation !== apngLoadGeneration) return;
        apngHdrPlayer.play(startedAt);
        if (reducedMotion) {
          await ensureAtlas();
          return;
        }
        show('apng');
        snapshot.readiness = 'ready';
        snapshot.settledTarget = 'spinner';
        notify();
      } catch {
        if (destroyed || generation !== apngLoadGeneration) return;
        apngHdrPlayer.disconnect();
        await ensureAtlas();
      }
    }

    show(reducedMotion ? 'atlas' : 'apng');
    notify();
    if (reducedMotion) void ensureAtlas();
    else void loadApng();

    return {
      destroy() {
        if (destroyed) return;
        destroyed = true;
        apngLoadGeneration += 1;
        apngHdrPlayer.disconnect();
        atlasPlayer?.dispose();
        listeners.clear();
        root.remove();
      },
      get snapshot() {
        return { ...snapshot };
      },
      subscribe(listener) {
        if (!destroyed) {
          listeners.add(listener);
          listener({ ...snapshot });
        }
        return () => listeners.delete(listener);
      },
      update(next) {
        if (destroyed || next.reducedMotion === undefined) return;
        const nextReducedMotion = Boolean(next.reducedMotion);
        if (nextReducedMotion === reducedMotion) return;
        reducedMotion = nextReducedMotion;
        if (reducedMotion) {
          void ensureAtlas().then(() => atlasPlayer?.setPhase(0));
        } else {
          void loadApng();
        }
      },
    };
  }

  return { mountObject, mountSpinner };
}
