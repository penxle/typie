import { createAdaptiveLightResolutionPolicy, resolvePrismRenderTargets, resolvePrismRenderWork } from './prism-render-policy.ts';
import { PRISM_SPINNER_HDR_HEADROOM_DEFAULT } from './prism-spinner-hdr.ts';
import {
  createPrismSpinnerMorphGeometry,
  nextForwardSpinnerSilhouettePhase,
  PRISM_SPINNER_CSS_SIZE,
  PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS,
  resolvePrismSpinnerMorphChannels,
} from './prism-spinner-morph.ts';
import { createDeferredPrismWgpuSurface } from './prism-wgpu-renderer.ts';
import type { PrismIconMorphSample } from './prism-icon-morph.ts';
import type { CreatePrismWebRenderer } from './prism-wgpu-renderer.ts';

type Quaternion = readonly [number, number, number, number];

const DEFAULT_SPINNER_MORPH_MATCH_PHASE = nextForwardSpinnerSilhouettePhase(0, 0.25);

function resizeCanvas(canvas: HTMLCanvasElement, maximumDpr = 2.5, resolutionScale = 1) {
  const rect = canvas.getBoundingClientRect();
  const dpr = Math.min(window.devicePixelRatio || 1, maximumDpr);
  const scale = Math.max(0.25, Math.min(Number(resolutionScale) || 1, 1));
  const pixelScale = Math.max(0.5, dpr * scale);
  const width = Math.max(1, Math.round(rect.width * pixelScale));
  const height = Math.max(1, Math.round(rect.height * pixelScale));
  const resized = canvas.width !== width || canvas.height !== height;
  if (resized) {
    canvas.width = width;
    canvas.height = height;
  }
  return { rect, dpr, resized };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.max(minimum, Math.min(maximum, value));
}

export function resolveSpectralFanReach(scatteringFalloff: number, renderScale: number): number {
  const scale = Math.max(renderScale || 0, 0.001);
  const attenuationDistance = Math.max((scatteringFalloff || 0) * scale, 0.02);
  return clamp(attenuationDistance * 6.5, 0.24 * scale, 3.2);
}

export function resolvePrismObjectRenderScale(
  iconMorphActive: boolean,
  transitionScale: number,
  prismScale: number,
  spinnerMorphScale: number,
): number {
  return iconMorphActive ? transitionScale : prismScale * Math.max(spinnerMorphScale, 0.001);
}

function normalizeSizeScaleOverride(value: unknown): number | null {
  if (value == null) return null;
  if (typeof value !== 'number' || !Number.isFinite(value) || value <= 0) {
    throw new RangeError('Prism size scale override must be finite and greater than zero.');
  }
  return Number(value);
}

function smoothstep(minimum: number, maximum: number, value: number): number {
  if (minimum === maximum) return value < minimum ? 0 : 1;
  const t = clamp((value - minimum) / (maximum - minimum), 0, 1);
  return t * t * (3 - 2 * t);
}

export function resolvePrismMaterialOpacityScale(transmission: number): number {
  const normalized = clamp(transmission || 0, 0, 2) / 2;
  const eased = normalized * normalized * (3 - 2 * normalized);
  return 1.18 + (0.7 - 1.18) * eased;
}

export function resolvePrismMorphHdrHeadroom(prismHeadroom: number, spinnerHeadroom: number, progress: number): number {
  const prism = clamp(prismHeadroom || 1.25, 1, 2.5);
  const spinner = clamp(spinnerHeadroom || PRISM_SPINNER_HDR_HEADROOM_DEFAULT, 1, 2.5);
  const normalized = clamp(progress || 0, 0, 1);
  const eased = normalized * normalized * normalized * (normalized * (normalized * 6 - 15) + 10);
  return prism + (spinner - prism) * eased;
}

let cssColorProbeContext: CanvasRenderingContext2D | null | undefined;

function decodeCssColor(color: string): readonly [number, number, number, number] {
  if (!cssColorProbeContext) {
    const probe = document.createElement('canvas');
    probe.width = 1;
    probe.height = 1;
    cssColorProbeContext = probe.getContext('2d', { willReadFrequently: true });
  }
  if (!cssColorProbeContext) return [1, 1, 1, 1];
  cssColorProbeContext.clearRect(0, 0, 1, 1);
  cssColorProbeContext.fillStyle = color;
  cssColorProbeContext.fillRect(0, 0, 1, 1);
  const [red, green, blue, alpha] = cssColorProbeContext.getImageData(0, 0, 1, 1).data;
  return [red / 255, green / 255, blue / 255, alpha / 255];
}

function backgroundLuminanceForElement(element: Element): number {
  let red = 0;
  let green = 0;
  let blue = 0;
  let alpha = 0;
  for (let current: Element | null = element; current instanceof Element && alpha < 0.999; current = current.parentElement) {
    const [layerRed, layerGreen, layerBlue, layerAlpha] = decodeCssColor(getComputedStyle(current).backgroundColor);
    const contribution = layerAlpha * (1 - alpha);
    red += layerRed * contribution;
    green += layerGreen * contribution;
    blue += layerBlue * contribution;
    alpha += contribution;
  }
  if (alpha < 0.999) {
    red += 1 - alpha;
    green += 1 - alpha;
    blue += 1 - alpha;
  }
  // Perceptual sRGB luminance is intentional here: this drives UI exposure,
  // not radiometric energy, so a 50% gray surface should adapt halfway.
  return clamp(red * 0.2126 + green * 0.7152 + blue * 0.0722, 0, 1);
}

function multiplyQuaternion(left: Quaternion, right: Quaternion): [number, number, number, number] {
  return [
    left[3] * right[0] + right[3] * left[0] + left[1] * right[2] - left[2] * right[1],
    left[3] * right[1] + right[3] * left[1] + left[2] * right[0] - left[0] * right[2],
    left[3] * right[2] + right[3] * left[2] + left[0] * right[1] - left[1] * right[0],
    left[3] * right[3] - left[0] * right[0] - left[1] * right[1] - left[2] * right[2],
  ];
}

export function resolvePrismObjectPoseQuaternion(
  phase: number,
  depthTransition: number,
  perspectiveTransition: number,
): [number, number, number, number] {
  const yHalfAngle = (phase || 0) * Math.PI;
  const xHalfAngle = -0.15 * (depthTransition || 0);
  const zHalfAngle = 0.05 * (perspectiveTransition || 0);
  const rotationY: Quaternion = [0, Math.sin(yHalfAngle), 0, Math.cos(yHalfAngle)];
  const rotationX: Quaternion = [Math.sin(xHalfAngle), 0, 0, Math.cos(xHalfAngle)];
  const rotationZ: Quaternion = [0, 0, Math.sin(zHalfAngle), Math.cos(zHalfAngle)];
  const quaternion = multiplyQuaternion(multiplyQuaternion(rotationY, rotationX), rotationZ);
  const length = Math.hypot(...quaternion) || 1;
  return quaternion.map((component) => component / length) as [number, number, number, number];
}

function resolveSettledPrismChannels() {
  return {
    geometry: [1, 0, 1, 1],
    appearance: [1, 1, 1, 0],
  };
}

const MAX_OPTICAL_PLANES = 20;

const BASIC_PRISM_PLANES = Object.freeze([
  [0, -1, 0, 0.49],
  [Math.sqrt(3) * 0.5, 0.5, 0, 0.49],
  [-Math.sqrt(3) * 0.5, 0.5, 0, 0.49],
  [0, 0, 1, 0.58],
  [0, 0, -1, 0.58],
]);

const PRISM_ICON_SAMPLE_SCALARS = [
  'cameraDistance',
  'edgeAlpha',
  'edgeHighlightProgress',
  'edgeWidthCssPixels',
  'optics',
  'phase',
  'progress',
  'projectionDistance',
  'renderScale',
  'spectrumPassProgress',
  'surface',
  'surfaceReveal',
  'velocity',
];

export function normalizePrismIconMorphSample(sample: unknown): PrismIconMorphSample {
  if (!sample || typeof sample !== 'object') throw new TypeError('Prism icon morph sample must be an object.');
  const record = sample as Record<string, unknown>;
  const required = [...PRISM_ICON_SAMPLE_SCALARS, 'angularVelocity', 'orientation'];
  if (required.some((field) => !Object.hasOwn(record, field))) {
    throw new TypeError('Prism icon morph sample must contain every Core field.');
  }
  const values = PRISM_ICON_SAMPLE_SCALARS.map((field) => record[field]);
  if (values.some((value) => !(typeof value === 'number' && Number.isFinite(value)))) {
    throw new RangeError('Prism icon morph sample fields must be finite.');
  }
  for (const field of ['edgeAlpha', 'edgeHighlightProgress', 'optics', 'progress', 'spectrumPassProgress', 'surface', 'surfaceReveal']) {
    const value = Number(record[field]);
    if (value < 0 || value > 1) throw new RangeError(`Prism icon ${field} must be between zero and one.`);
  }
  if (
    Number(record.cameraDistance) <= 0 ||
    Number(record.projectionDistance) <= 0 ||
    Number(record.renderScale) <= 0 ||
    Number(record.edgeWidthCssPixels) <= 0 ||
    Number(record.velocity) < 0
  ) {
    throw new RangeError('Prism icon projection, scale, edge width, and forward velocity must be valid.');
  }
  const orientation = record.orientation;
  if (!Array.isArray(orientation) || orientation.length !== 4 || !orientation.every(Number.isFinite) || Math.hypot(...orientation) === 0) {
    throw new RangeError('Prism icon orientation must be a finite, non-zero quaternion.');
  }
  const angularVelocity = record.angularVelocity;
  if (!Array.isArray(angularVelocity) || angularVelocity.length !== 3 || !angularVelocity.every(Number.isFinite)) {
    throw new RangeError('Prism icon angular velocity must contain three finite numbers.');
  }
  return {
    ...(record as Omit<PrismIconMorphSample, 'angularVelocity' | 'orientation'>),
    angularVelocity: [angularVelocity[0], angularVelocity[1], angularVelocity[2]],
    orientation: [orientation[0], orientation[1], orientation[2], orientation[3]],
  } as PrismIconMorphSample;
}

export function normalizePrismIconEdgeColor(color: unknown): [number, number, number, number] {
  if (!Array.isArray(color) || color.length !== 4) {
    throw new TypeError('Prism icon edge color must contain four normalized display-sRGB components.');
  }
  if (color.some((component) => !(typeof component === 'number' && Number.isFinite(component)))) {
    throw new RangeError('Prism icon edge color must be finite.');
  }
  return [clamp(Number(color[0]), 0, 1), clamp(Number(color[1]), 0, 1), clamp(Number(color[2]), 0, 1), clamp(Number(color[3]), 0, 1)];
}

function createPrismPlanes(): { count: number; values: Float32Array } {
  const values = new Float32Array(MAX_OPTICAL_PLANES * 4);
  for (const [planeIndex, plane] of BASIC_PRISM_PLANES.entries()) values.set(plane, planeIndex * 4);
  return { count: BASIC_PRISM_PLANES.length, values };
}

export function resolvePrismObjectPlanes(spinnerMorphProgress = 0): { count: number; values: Float32Array } {
  const morphProgress = clamp(Number(spinnerMorphProgress) || 0, 0, 1);
  if (morphProgress < PRISM_SPINNER_GEOMETRY_SWAP_PROGRESS) return createPrismPlanes();

  const geometry = createPrismSpinnerMorphGeometry(morphProgress);
  const values = new Float32Array(MAX_OPTICAL_PLANES * 4);
  values.set(geometry.planes);
  return { count: geometry.planeCount, values };
}

export const prismObjectDefaults = {
  canvasSize: 160,
  prismSize: 72,
  transitionProgress: 0,
  iconSize: 32,
  iconEdgeColor: [0.5, 0.5, 0.5, 1],
  spinnerMorphProgress: 0,
  spinnerSize: PRISM_SPINNER_CSS_SIZE,
  sizeScaleOverride: null,
  maxFps: 60,
  period: 3.9,
  phase: 0.04,
  lightPeriod: 3.6,
  lightPhaseOffset: 0.23,
  lightCount: 3,
  lightRadius: 1.25,
  sourceSize: 0.31,
  sourceDivergence: 0.62,
  sourceHalo: 2.05,
  sourceSampleCount: 5,
  rayleighMix: 0.04,
  causticHalo: 1.45,
  turbulenceStrength: 0.34,
  turbulenceSpeed: 0.14,
  lightResolutionScale: 0.25,
  incidentStrength: 0.9,
  scatteringStrength: 1.45,
  scatteringFalloff: 0.19,
  beamWidth: 1.3,
  ior: 1.6,
  dispersion: 1.25,
  transmission: 1.7,
  lightThroughput: 1,
  roughness: 0.08,
  fresnelStrength: 1,
  sheenStrength: 1.15,
  sheenWidth: 2.35,
  sheenChroma: 0.9,
  visibility: 0.9,
  bevel: 0.07,
  environmentLuminance: null,
  hdr: 'auto',
  hdrHeadroom: 1.25,
  spinnerHdrHeadroom: PRISM_SPINNER_HDR_HEADROOM_DEFAULT,
};

type PrismObjectOptions = Omit<typeof prismObjectDefaults, 'environmentLuminance' | 'hdr' | 'iconEdgeColor' | 'sizeScaleOverride'> & {
  createRenderer?: CreatePrismWebRenderer;
  environmentLuminance: number | null;
  hdr: 'auto' | 'off' | 'on';
  iconEdgeColor: [number, number, number, number];
  sizeScaleOverride: number | null;
  spinnerMorphMatchPhase?: number;
};

type PrismObjectOptionsUpdate = Partial<PrismObjectOptions>;

export const PRISM_RENDER_SCALE_REFERENCE_SIZE = 92;
const PRISM_OBJECT_REFERENCE_SIZE = 132;
const SPINNER_OPTICAL_SCALE_CORRECTION = (0.876 * PRISM_RENDER_SCALE_REFERENCE_SIZE) / (PRISM_OBJECT_REFERENCE_SIZE * 0.98);

function createUnavailablePrismObjectController(canvas: HTMLCanvasElement, options: PrismObjectOptions) {
  let rotationPhase = Number(options.phase) || 0;
  canvas.parentElement?.setAttribute('data-prism-object-fallback', '');
  return {
    onUnavailable(callback: (error: unknown) => void) {
      callback(new Error('WebGPU is unavailable.'));
      return () => {
        // The unavailable controller never retains listeners.
      };
    },
    setActive(nextActive: boolean) {
      if (!nextActive) return;
      // The fallback has no renderer to activate when enabled.
    },
    setRotationPhase(nextPhase: number) {
      if (Number.isFinite(nextPhase)) rotationPhase = nextPhase;
    },
    setMorphPoseQuaternion(nextQuaternion: Quaternion | null) {
      if (nextQuaternion === null) return;
      // The fallback retains only its scalar rotation phase.
    },
    setIconMorphSample(nextSample: PrismIconMorphSample | null) {
      if (nextSample !== null) normalizePrismIconMorphSample(nextSample);
    },
    resumeRotation() {
      // The fallback does not schedule animation frames.
    },
    update(nextOptions: PrismObjectOptionsUpdate = {}) {
      if ('iconEdgeColor' in nextOptions) {
        nextOptions = {
          ...nextOptions,
          iconEdgeColor: normalizePrismIconEdgeColor(nextOptions.iconEdgeColor),
        };
      }
      Object.assign(options, nextOptions);
    },
    whenReady() {
      return Promise.resolve('unavailable');
    },
    whenFirstFramePresented() {
      return Promise.resolve();
    },
    get readiness() {
      return 'unavailable';
    },
    get rotationPhase() {
      return rotationPhase;
    },
    get poseSnapshot() {
      const velocity = 1 / Math.max(Number(options.period) || 3.6, 0.1);
      return {
        angularVelocity: [0, Math.PI * 2 * velocity, 0] as const,
        orientation: resolvePrismObjectPoseQuaternion(rotationPhase, 1, 1),
        phase: rotationPhase,
        velocity,
      };
    },
    dispose() {
      // The fallback owns no GPU or observer resources.
    },
  };
}

export function mountPrismObject(canvas: HTMLCanvasElement, initialOptions: PrismObjectOptionsUpdate = {}) {
  if (!(canvas instanceof HTMLCanvasElement)) {
    throw new TypeError('mountPrismObject expects a canvas.');
  }

  const options: PrismObjectOptions = {
    ...prismObjectDefaults,
    ...initialOptions,
    hdr: initialOptions.hdr ?? 'auto',
    iconEdgeColor: normalizePrismIconEdgeColor(initialOptions.iconEdgeColor ?? prismObjectDefaults.iconEdgeColor),
    sizeScaleOverride: normalizeSizeScaleOverride(initialOptions.sizeScaleOverride),
  };
  const spinnerElement = canvas.closest<HTMLElement>('.prism-object');
  const previousSpinnerSize = spinnerElement?.style.getPropertyValue('--prism-object-size') ?? '';
  function applySpinnerSize() {
    const canvasSize = clamp(Number(options.canvasSize) || PRISM_RENDER_SCALE_REFERENCE_SIZE, 72, 320);
    spinnerElement?.style.setProperty('--prism-object-size', `${canvasSize}px`);
  }
  applySpinnerSize();
  let surfaceCandidate: ReturnType<typeof createDeferredPrismWgpuSurface> | null = null;
  try {
    if (options.createRenderer && globalThis.navigator?.gpu) {
      surfaceCandidate = createDeferredPrismWgpuSurface(canvas, options.createRenderer);
    }
  } catch (err) {
    console.warn('[Typie Prism] prism spinner shader failed.', err);
    surfaceCandidate?.dispose();
    return createUnavailablePrismObjectController(canvas, options);
  }
  if (!surfaceCandidate) {
    return createUnavailablePrismObjectController(canvas, options);
  }
  const surface = surfaceCandidate;
  let destroyed = false;
  let visible = true;
  let active = true;
  let frameId = 0;
  let lastFrameTime = performance.now();
  let rotationPhase = options.transitionProgress > 0.0001 ? options.phase : 0;
  let relativeLightPhase = options.phase;
  let rotationLocked = options.transitionProgress <= 0.0001;
  let spinnerMorphPoseQuaternion: Quaternion | null = null;
  let iconMorphSample: PrismIconMorphSample | null = null;
  let nextRenderAt = 0;
  let environmentFrameId = 0;
  let environmentTrackingUntil = 0;
  let prismGeometryKey = '';
  let prismPlanes = resolvePrismObjectPlanes();
  let unavailableError: unknown;
  const unavailableListeners = new Set<(error: unknown) => void>();
  const lightResolutionPolicy = createAdaptiveLightResolutionPolicy(options.lightResolutionScale);
  const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
  let environmentLuminance = Number.isFinite(options.environmentLuminance)
    ? clamp(Number(options.environmentLuminance), 0, 1)
    : backgroundLuminanceForElement(canvas);

  surface.setHdrMode(options.hdr);
  surface.onReady((error) => {
    if (destroyed) return;
    if (error) canvas.parentElement?.setAttribute('data-prism-object-fallback', '');
    else schedule();
  });
  surface.onUnavailable((error) => {
    if (destroyed) return;
    unavailableError = error;
    active = false;
    if (frameId) cancelAnimationFrame(frameId);
    frameId = 0;
    canvas.parentElement?.setAttribute('data-prism-object-fallback', '');
    for (const listener of unavailableListeners) listener(error);
    unavailableListeners.clear();
  });

  function updateEnvironmentLuminance() {
    environmentLuminance = Number.isFinite(options.environmentLuminance)
      ? clamp(Number(options.environmentLuminance), 0, 1)
      : backgroundLuminanceForElement(canvas);
    schedule();
  }

  function onEnvironmentTransitionEnd(event: TransitionEvent) {
    if (event.propertyName === 'background-color' && event.target instanceof Element && event.target.contains(canvas)) {
      updateEnvironmentLuminance();
    }
  }

  function sampleEnvironmentTransition(now: number) {
    environmentFrameId = 0;
    updateEnvironmentLuminance();
    if (!destroyed && now < environmentTrackingUntil) {
      environmentFrameId = requestAnimationFrame(sampleEnvironmentTransition);
    }
  }

  function trackEnvironmentTransition() {
    environmentTrackingUntil = performance.now() + 260;
    if (!environmentFrameId) {
      environmentFrameId = requestAnimationFrame(sampleEnvironmentTransition);
    }
  }

  function render(now: number) {
    frameId = 0;
    if (destroyed || !active || !visible || document.hidden) return;
    const maximumFps = clamp(Number(options.maxFps) || 60, 1, 120);
    const frameInterval = 1000 / maximumFps;
    if (!reducedMotion.matches) {
      if (nextRenderAt > 0 && now + 0.25 < nextRenderAt) {
        schedule();
        return;
      }
      nextRenderAt = (nextRenderAt > 0 ? nextRenderAt : now) + frameInterval;
      if (nextRenderAt < now) nextRenderAt = now + frameInterval;
    }
    const measuredFrameInterval = Math.max(now - lastFrameTime, 0);
    const qualityFrameInterval = 1000 / Math.min(maximumFps, 60);
    let lightResolutionScale = options.lightResolutionScale;
    if (!reducedMotion.matches) {
      lightResolutionScale = lightResolutionPolicy.sample(measuredFrameInterval, qualityFrameInterval, options.lightResolutionScale);
    }
    const { rect: canvasRect } = resizeCanvas(canvas, 3);
    const renderTargets = resolvePrismRenderTargets(canvas.width, canvas.height, lightResolutionScale, lightResolutionPolicy.materialScale);
    surface.resizeLightTarget(renderTargets.light.width, renderTargets.light.height);
    surface.resizeMaterialTarget(renderTargets.material.width, renderTargets.material.height);
    const elapsedSeconds = Math.min(Math.max((now - lastFrameTime) / 1000, 0), 0.08);
    lastFrameTime = now;
    if (!rotationLocked && !reducedMotion.matches) {
      rotationPhase += elapsedSeconds / Math.max(Number(options.period) || 3.6, 0.1);
      relativeLightPhase +=
        elapsedSeconds * (1 / Math.max(Number(options.lightPeriod) || 3.6, 0.1) - 1 / Math.max(Number(options.period) || 3.6, 0.1));
    }
    const phase = iconMorphSample?.phase ?? ((rotationPhase % 1) + 1) % 1;
    const lightPhase = (((phase + relativeLightPhase + options.lightPhaseOffset) % 1) + 1) % 1;
    const spinnerMorphChannels = resolvePrismSpinnerMorphChannels(
      iconMorphSample ? 0 : options.spinnerMorphProgress,
      options.prismSize,
      options.spinnerSize,
    );
    const renderWork = resolvePrismRenderWork(spinnerMorphChannels.geometry, iconMorphSample);
    const nextGeometryKey = spinnerMorphChannels.geometry.toFixed(6);
    if (nextGeometryKey !== prismGeometryKey) {
      prismGeometryKey = nextGeometryKey;
      prismPlanes = resolvePrismObjectPlanes(spinnerMorphChannels.geometry);
    }
    const viewportScale = canvasRect.height / PRISM_OBJECT_REFERENCE_SIZE;
    const prismScale = clamp(Number(options.prismSize) || PRISM_RENDER_SCALE_REFERENCE_SIZE, 48, 160) / PRISM_RENDER_SCALE_REFERENCE_SIZE;
    const transitionChannels = iconMorphSample
      ? {
          geometry: [
            iconMorphSample.renderScale,
            iconMorphSample.edgeWidthCssPixels,
            iconMorphSample.edgeHighlightProgress,
            iconMorphSample.surfaceReveal,
          ],
          appearance: [iconMorphSample.surface, iconMorphSample.optics, iconMorphSample.spectrumPassProgress, iconMorphSample.edgeAlpha],
        }
      : resolveSettledPrismChannels();
    const resolvedIconSize = clamp(Number(options.iconSize) || 32, 24, 160);
    const spinnerScaleCorrection = Math.exp(Math.log(SPINNER_OPTICAL_SCALE_CORRECTION) * spinnerMorphChannels.camera);
    const spinnerMorphScale =
      (spinnerMorphChannels.size / Math.max(Number(options.prismSize) || PRISM_RENDER_SCALE_REFERENCE_SIZE, 1)) *
      spinnerScaleCorrection *
      spinnerMorphChannels.geometryScale;
    const sizeScaleOverride = normalizeSizeScaleOverride(options.sizeScaleOverride);
    const spinnerMorphCorrection = spinnerScaleCorrection * spinnerMorphChannels.geometryScale;
    const transitionPrismScale =
      sizeScaleOverride === null
        ? iconMorphSample
          ? iconMorphSample.renderScale
          : prismScale * spinnerMorphScale
        : sizeScaleOverride * (iconMorphSample ? 1 : spinnerMorphCorrection);
    const renderPrismScale =
      sizeScaleOverride === null
        ? resolvePrismObjectRenderScale(Boolean(iconMorphSample), transitionPrismScale, prismScale, spinnerMorphScale)
        : transitionPrismScale;
    const spinnerCameraDepth = 0.11 / 0.3;
    const opticalDepthTransition = 1 + (spinnerCameraDepth - 1) * spinnerMorphChannels.camera;
    const opticalPerspectiveTransition = 1 + (-0.3 - 1) * spinnerMorphChannels.camera;
    const resolvedPoseQuaternion =
      iconMorphSample?.orientation ??
      spinnerMorphPoseQuaternion ??
      (spinnerMorphChannels.alignment < 0.000001 && spinnerMorphChannels.material < 0.000001
        ? resolvePrismObjectPoseQuaternion(phase, opticalDepthTransition, opticalPerspectiveTransition)
        : null);
    const opticalSourceSampleCount = Math.min(clamp(Math.round(options.sourceSampleCount), 1, 5), lightResolutionPolicy.sourceSampleCap);
    const opticalFrame = {
      planes: prismPlanes.values,
      planeCount: prismPlanes.count,
      transitionScale: transitionPrismScale,
      prismScale: renderPrismScale,
      bevel: options.bevel,
      phase,
      lightPhase,
      depthTransition: opticalDepthTransition,
      perspectiveTransition: opticalPerspectiveTransition,
      lightCount: options.lightCount,
      lightRadius: options.lightRadius,
      sourceSize: options.sourceSize,
      sourceDivergence: options.sourceDivergence,
      sourceSampleCount: opticalSourceSampleCount,
      ior: options.ior,
      dispersion: options.dispersion,
      enabled: renderWork.opticalPaths,
    };
    surface.setOpticalFrame(opticalFrame);
    const spinnerMorphMatchPhase = Number(options.spinnerMorphMatchPhase) || DEFAULT_SPINNER_MORPH_MATCH_PHASE;
    const lightThroughput = clamp(Number(options.lightThroughput) || 0, 0, 1);
    const materialOpacityScale = resolvePrismMaterialOpacityScale(options.transmission);
    function prepareSurface(width: number, height: number, renderLayer: number) {
      const uniforms = surface.frameUniforms;
      uniforms.setVector2('uResolution', width, height);
      uniforms.setFloat('uPhase', phase);
      uniforms.setFloat('uLightPhase', lightPhase);
      uniforms.setInteger('uLightCount', clamp(Math.round(options.lightCount), 1, 3));
      uniforms.setInteger('uRenderLayer', renderLayer);
      uniforms.setFloat('uIor', options.ior);
      uniforms.setFloat('uDispersion', options.dispersion);
      uniforms.setFloat('uTransmission', options.transmission);
      uniforms.setFloat('uLightThroughput', lightThroughput);
      uniforms.setFloat('uMaterialOpacityScale', materialOpacityScale);
      uniforms.setFloat('uRoughness', options.roughness);
      uniforms.setFloat('uFresnelStrength', options.fresnelStrength);
      uniforms.setFloat('uSheenStrength', options.sheenStrength);
      uniforms.setFloat('uSheenWidth', options.sheenWidth);
      uniforms.setFloat('uSheenChroma', options.sheenChroma);
      uniforms.setFloat('uVisibility', options.visibility);
      uniforms.setFloat('uBevel', options.bevel);
      uniforms.setVector4Array('uPrismPlanes[0]', prismPlanes.values);
      uniforms.setInteger('uPrismPlaneCount', prismPlanes.count);
      uniforms.setFloat('uLightRadius', options.lightRadius);
      uniforms.setFloat('uSourceSize', options.sourceSize);
      uniforms.setFloat('uSourceDivergence', options.sourceDivergence);
      uniforms.setFloat('uSourceHalo', options.sourceHalo);
      const sourceSampleCount =
        renderLayer === 0
          ? Math.min(clamp(Math.round(options.sourceSampleCount), 1, 5), lightResolutionPolicy.sourceSampleCap)
          : clamp(Math.round(options.sourceSampleCount), 1, 5);
      uniforms.setInteger('uSourceSampleCount', sourceSampleCount);
      uniforms.setFloat('uRayleighMix', options.rayleighMix);
      uniforms.setFloat('uCausticHalo', options.causticHalo);
      uniforms.setFloat('uTurbulenceStrength', options.turbulenceStrength);
      uniforms.setFloat('uTurbulenceSpeed', options.turbulenceSpeed);
      uniforms.setFloat('uOpticalTime', reducedMotion.matches ? 0 : now / 1000);
      uniforms.setFloat('uIncidentStrength', options.incidentStrength);
      uniforms.setFloat('uScatteringStrength', options.scatteringStrength);
      uniforms.setFloat('uScatteringFalloff', options.scatteringFalloff);
      uniforms.setFloat('uSpectralFanReach', resolveSpectralFanReach(options.scatteringFalloff, renderPrismScale));
      uniforms.setFloat('uBeamWidth', options.beamWidth);
      const [geometryScale, edgeWidth, edgeHighlight, surfaceReveal] = transitionChannels.geometry;
      uniforms.setVector4('uTransitionGeometry', geometryScale, edgeWidth, edgeHighlight, surfaceReveal);
      const [surfaceOpacity, optics, spectrumPass, edgeAlpha] = transitionChannels.appearance;
      uniforms.setVector4('uTransitionAppearance', surfaceOpacity, optics, spectrumPass, edgeAlpha);
      uniforms.setFloat('uIconSize', resolvedIconSize);
      uniforms.setFloat('uViewportScale', viewportScale);
      uniforms.setFloat('uPrismScale', prismScale);
      uniforms.setFloat('uTransitionPrismScale', transitionPrismScale);
      uniforms.setVector4(
        'uSpinnerMorph',
        spinnerMorphChannels.alignment,
        spinnerMorphChannels.camera,
        spinnerMorphChannels.geometry,
        spinnerMorphChannels.material,
      );
      uniforms.setVector2('uSpinnerMaterialMorph', spinnerMorphChannels.edge, spinnerMorphChannels.sheen);
      uniforms.setFloat('uSpinnerMorphScale', spinnerMorphScale);
      uniforms.setFloat('uSpinnerFrameSize', spinnerMorphChannels.size);
      uniforms.setFloat('uCssPixelRatio', height / Math.max(canvasRect.height, 1));
      uniforms.setFloat('uSpinnerMorphMatchPhase', spinnerMorphMatchPhase);
      uniforms.setVector4('uObjectPoseQuaternion', ...(resolvedPoseQuaternion ?? [0, 0, 0, 1]));
      uniforms.setFloat('uObjectPoseOverride', resolvedPoseQuaternion ? 1 : 0);
      uniforms.setFloat('uSpinnerCreaseMorph', spinnerMorphChannels.crease);
      uniforms.setInteger('uMaterialSpectralSampleCount', lightResolutionPolicy.materialSpectralSampleCount);
      const [spectralRed = 1, spectralGreen = 1, spectralBlue = 1] = lightResolutionPolicy.materialSpectralNormalization;
      uniforms.setVector3('uMaterialSpectralNormalization', spectralRed, spectralGreen, spectralBlue);
      uniforms.setFloat('uDarkMode', document.documentElement.dataset.theme === 'dark' ? 1 : 0);
      uniforms.setFloat('uEnvironmentLuminance', environmentLuminance);
      uniforms.setFloat('uEnvironmentLightMix', smoothstep(0.18, 0.82, environmentLuminance));
      uniforms.setFloat('uRenderPrismScale', renderPrismScale);
      uniforms.setFloat('uScaledScatteringFalloff', options.scatteringFalloff * renderPrismScale);
      uniforms.setVector4('uIconEdgeColor', ...options.iconEdgeColor);
      uniforms.setVector4(
        'uObjectProjection',
        iconMorphSample?.cameraDistance ?? 4.2,
        iconMorphSample?.projectionDistance ?? 2.96,
        1,
        iconMorphSample ? 1 : 0,
      );
      let scissor: { height: number; width: number; x: number; y: number } | null = null;
      if (renderLayer === 1) {
        const backingScale = height / Math.max(canvasRect.height, 1);
        const extent = Math.min(
          height,
          Math.ceil(clamp(iconMorphSample ? options.prismSize : spinnerMorphChannels.size, 18, 160) * 1.55 * backingScale),
        );
        scissor = {
          x: Math.floor((width - extent) * 0.5),
          y: Math.floor((height - extent) * 0.5),
          width: extent,
          height: extent,
        };
      }
      return scissor;
    }
    if (renderWork.light) {
      // The shared renderer derives light uniforms from the material frame and
      // overrides only the pass-specific values in Rust.
      surface.drawLight(opticalSourceSampleCount);
    } else {
      surface.clearLight();
    }
    const prismScissor = prepareSurface(renderTargets.material.width, renderTargets.material.height, 1);
    surface.drawPrism(prismScissor);
    const hdrHeadroom = resolvePrismMorphHdrHeadroom(options.hdrHeadroom, options.spinnerHdrHeadroom, options.spinnerMorphProgress);
    surface.setHdrHeadroom(options.hdr === 'off' ? 1 : hdrHeadroom);
    surface.compositeSdr();
    if (!reducedMotion.matches) schedule();
  }

  function schedule() {
    if (!frameId && !destroyed && active && visible && !document.hidden && surface.readiness === 'ready') {
      frameId = requestAnimationFrame(render);
    }
  }

  const resizeObserver = new ResizeObserver(schedule);
  resizeObserver.observe(canvas);
  const intersectionObserver = new IntersectionObserver(
    ([entry]) => {
      visible = entry?.isIntersecting ?? true;
      if (visible) schedule();
    },
    { rootMargin: '100px' },
  );
  intersectionObserver.observe(canvas);
  document.addEventListener('visibilitychange', schedule);
  document.addEventListener('transitionend', onEnvironmentTransitionEnd);
  window.addEventListener('typie-prism-themechange', trackEnvironmentTransition);
  reducedMotion.addEventListener?.('change', schedule);
  schedule();

  return {
    onUnavailable(callback: (error: unknown) => void) {
      if (surface.readiness === 'unavailable') callback(unavailableError);
      else unavailableListeners.add(callback);
      return () => unavailableListeners.delete(callback);
    },
    setActive(nextActive: boolean) {
      active = nextActive;
      if (!active && frameId) {
        cancelAnimationFrame(frameId);
        frameId = 0;
      }
      if (active) {
        lastFrameTime = performance.now();
        nextRenderAt = 0;
        schedule();
      }
    },
    setRotationPhase(nextPhase: number) {
      if (Number.isFinite(nextPhase)) rotationPhase = nextPhase;
      rotationLocked = true;
      lastFrameTime = performance.now();
      schedule();
    },
    setMorphPoseQuaternion(nextQuaternion: Quaternion | null) {
      if (nextQuaternion == null) {
        spinnerMorphPoseQuaternion = null;
      } else if (nextQuaternion.length === 4 && [...nextQuaternion].every(Number.isFinite)) {
        spinnerMorphPoseQuaternion = [...nextQuaternion];
      } else {
        throw new TypeError('Morph pose quaternion must contain four finite numbers.');
      }
      schedule();
    },
    setIconMorphSample(nextSample: PrismIconMorphSample | null) {
      iconMorphSample = nextSample === null ? null : normalizePrismIconMorphSample(nextSample);
      if (iconMorphSample) {
        rotationPhase = iconMorphSample.phase;
        rotationLocked = true;
      }
      lastFrameTime = performance.now();
      schedule();
    },
    resumeRotation() {
      rotationLocked = false;
      lastFrameTime = performance.now();
      schedule();
    },
    update(nextOptions: PrismObjectOptionsUpdate = {}) {
      const previousCanvasSize = options.canvasSize;
      const previousEnvironmentLuminance = options.environmentLuminance;
      const previousLightResolutionScale = options.lightResolutionScale;
      const previousHdr = options.hdr;
      const normalizedOptions =
        'iconEdgeColor' in nextOptions
          ? {
              ...nextOptions,
              iconEdgeColor: normalizePrismIconEdgeColor(nextOptions.iconEdgeColor),
            }
          : nextOptions;
      if ('sizeScaleOverride' in normalizedOptions) {
        normalizedOptions.sizeScaleOverride = normalizeSizeScaleOverride(normalizedOptions.sizeScaleOverride);
      }
      Object.assign(options, normalizedOptions);
      if (options.canvasSize !== previousCanvasSize) applySpinnerSize();
      if (options.lightResolutionScale !== previousLightResolutionScale) {
        lightResolutionPolicy.reset(options.lightResolutionScale);
      }
      if (options.hdr !== previousHdr) {
        surface.setHdrMode(options.hdr);
      }
      if (options.environmentLuminance === previousEnvironmentLuminance) {
        schedule();
      } else {
        updateEnvironmentLuminance();
      }
    },
    whenReady() {
      return surface.whenReady();
    },
    whenFirstFramePresented() {
      return surface.whenFirstFramePresented();
    },
    get readiness() {
      return surface.readiness;
    },
    get rotationPhase() {
      return rotationPhase;
    },
    get poseSnapshot() {
      const velocity = 1 / Math.max(Number(options.period) || 3.6, 0.1);
      return {
        angularVelocity: [0, Math.PI * 2 * velocity, 0] as const,
        orientation: resolvePrismObjectPoseQuaternion(rotationPhase, 1, 1),
        phase: rotationPhase,
        velocity,
      };
    },
    dispose() {
      destroyed = true;
      if (frameId) cancelAnimationFrame(frameId);
      if (environmentFrameId) cancelAnimationFrame(environmentFrameId);
      resizeObserver.disconnect();
      intersectionObserver.disconnect();
      document.removeEventListener('visibilitychange', schedule);
      document.removeEventListener('transitionend', onEnvironmentTransitionEnd);
      window.removeEventListener('typie-prism-themechange', trackEnvironmentTransition);
      reducedMotion.removeEventListener?.('change', schedule);
      unavailableListeners.clear();
      surface.dispose();
      if (spinnerElement) {
        if (previousSpinnerSize) {
          spinnerElement.style.setProperty('--prism-object-size', previousSpinnerSize);
        } else {
          spinnerElement.style.removeProperty('--prism-object-size');
        }
      }
    },
  };
}
