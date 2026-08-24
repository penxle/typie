import type { PrismIconMorphSample } from './prism-icon-morph.ts';

export type PrismMode = 'icon' | 'prism' | 'spinner';
export type PrismModeOwner = 'svg' | 'webgpu' | 'atlas';
export type PrismModeReadiness = 'loading' | 'ready' | 'unavailable';

type SharedKinematics = {
  angularVelocity?: readonly [number, number, number];
  orientation?: readonly [number, number, number, number];
  phase: number;
  velocity: number;
};

export type PrismModeRouteAction = {
  instant: boolean;
  kind: 'icon' | 'spinner';
  startAngularVelocity?: readonly [number, number, number];
  startEdgeHighlightProgress?: number;
  startOrientation?: readonly [number, number, number, number];
  startPhase: number;
  startProgress: number;
  startVelocity: number;
  targetProgress: 0 | 1;
};

type RouteSample = SharedKinematics & { edgeHighlightProgress?: number; progress: number };

const EPSILON = 0.0001;

function requireProgress(progress: number) {
  if (!Number.isFinite(progress) || progress < 0 || progress > 1) {
    throw new RangeError('Prism route progress must be finite and between zero and one.');
  }
}

function normalizeSample(sample: RouteSample): RouteSample {
  requireProgress(sample.progress);
  if (!Number.isFinite(sample.phase) || !Number.isFinite(sample.velocity) || sample.velocity < 0) {
    throw new RangeError('Prism route phase and forward velocity must be finite.');
  }
  if (
    sample.edgeHighlightProgress !== undefined &&
    (!Number.isFinite(sample.edgeHighlightProgress) || sample.edgeHighlightProgress < 0 || sample.edgeHighlightProgress > 1)
  )
    throw new RangeError('Prism route edge highlight progress must be between zero and one.');
  if (sample.orientation !== undefined && (sample.orientation.length !== 4 || !sample.orientation.every(Number.isFinite)))
    throw new RangeError('Prism route orientation must contain four finite numbers.');
  if (sample.angularVelocity !== undefined && (sample.angularVelocity.length !== 3 || !sample.angularVelocity.every(Number.isFinite)))
    throw new RangeError('Prism route angular velocity must contain three finite numbers.');
  return {
    ...sample,
    angularVelocity: sample.angularVelocity ? ([...sample.angularVelocity] as [number, number, number]) : undefined,
    orientation: sample.orientation ? ([...sample.orientation] as [number, number, number, number]) : undefined,
  };
}

export function createPrismModeRoute(
  options: {
    iconProgress?: number;
    reducedMotion?: boolean;
    rendererReadiness?: PrismModeReadiness;
    spinnerProgress?: number;
    spinnerReadiness?: PrismModeReadiness;
  } = {},
) {
  let requestedMode: PrismMode = 'icon';
  let rendererReadiness = options.rendererReadiness ?? 'loading';
  let spinnerReadiness = options.spinnerReadiness ?? 'loading';
  let reducedMotion = Boolean(options.reducedMotion);
  let iconProgress = options.iconProgress ?? 0;
  let spinnerProgress = options.spinnerProgress ?? 0;
  requireProgress(iconProgress);
  requireProgress(spinnerProgress);
  let owner: PrismModeOwner = iconProgress <= EPSILON ? 'svg' : 'webgpu';
  let activeAction: PrismModeRouteAction | null = null;
  let shared: SharedKinematics = { phase: 0, velocity: 0 };
  let edgeHighlightProgress: number | undefined;

  function action(kind: 'icon' | 'spinner', startProgress: number, targetProgress: 0 | 1): PrismModeRouteAction {
    return {
      instant: reducedMotion,
      kind,
      startAngularVelocity: shared.angularVelocity,
      startEdgeHighlightProgress: edgeHighlightProgress,
      startOrientation: shared.orientation,
      startPhase: shared.phase,
      startProgress,
      startVelocity: shared.velocity,
      targetProgress,
    };
  }

  function nextAction(): PrismModeRouteAction | null {
    if (activeAction) return null;
    if (requestedMode === 'spinner') {
      if (iconProgress < 1 - EPSILON) {
        return rendererReadiness === 'ready' ? action('icon', iconProgress, 1) : null;
      }
      if (owner === 'atlas' && spinnerProgress >= 1 - EPSILON) return null;
      return spinnerReadiness === 'ready' ? action('spinner', spinnerProgress, 1) : null;
    }
    if (owner === 'atlas' || spinnerProgress > EPSILON) {
      return action('spinner', spinnerProgress, 0);
    }
    if (requestedMode === 'icon') {
      if (iconProgress <= EPSILON) return null;
      return rendererReadiness === 'ready' ? action('icon', iconProgress, 0) : null;
    }
    if (iconProgress >= 1 - EPSILON) return null;
    return rendererReadiness === 'ready' ? action('icon', iconProgress, 1) : null;
  }

  function recordSample(sample: RouteSample) {
    const normalized = normalizeSample(sample);
    if (normalized.edgeHighlightProgress !== undefined) {
      edgeHighlightProgress = normalized.edgeHighlightProgress;
    }
    shared = {
      angularVelocity: normalized.angularVelocity,
      orientation: normalized.orientation,
      phase: normalized.phase,
      velocity: normalized.velocity,
    };
    return normalized;
  }

  return {
    request(mode: PrismMode) {
      if (mode !== 'icon' && mode !== 'prism' && mode !== 'spinner') {
        throw new RangeError('Unknown Prism mode.');
      }
      requestedMode = mode;
      activeAction = null;
    },
    setRendererReadiness(readiness: PrismModeReadiness) {
      rendererReadiness = readiness;
      if (readiness === 'unavailable' && iconProgress <= EPSILON) owner = 'svg';
    },
    setSpinnerReadiness(readiness: PrismModeReadiness) {
      spinnerReadiness = readiness;
    },
    setReducedMotion(value: boolean) {
      reducedMotion = value;
    },
    syncKinematics(sample: SharedKinematics) {
      const normalized = normalizeSample({ ...sample, progress: iconProgress });
      shared = {
        angularVelocity: normalized.angularVelocity,
        orientation: normalized.orientation,
        phase: normalized.phase,
        velocity: normalized.velocity,
      };
    },
    syncState(next: { iconProgress?: number; owner?: PrismModeOwner; spinnerProgress?: number }) {
      if (next.iconProgress !== undefined) {
        requireProgress(next.iconProgress);
        iconProgress = next.iconProgress;
      }
      if (next.spinnerProgress !== undefined) {
        requireProgress(next.spinnerProgress);
        spinnerProgress = next.spinnerProgress;
      }
      if (next.owner !== undefined) {
        if (next.owner !== 'svg' && next.owner !== 'webgpu' && next.owner !== 'atlas') {
          throw new RangeError('Unknown Prism route owner.');
        }
        owner = next.owner;
      }
    },
    nextAction,
    begin(next: PrismModeRouteAction) {
      const expected = nextAction();
      if (!expected || expected.kind !== next.kind || expected.targetProgress !== next.targetProgress) {
        throw new Error('Prism route action is stale or another transition is active.');
      }
      activeAction = { ...next };
    },
    recordIconSample(sample: PrismIconMorphSample | RouteSample) {
      const normalized = recordSample(sample);
      iconProgress = normalized.progress;
      if (activeAction?.kind === 'icon' && activeAction.targetProgress === 1) owner = 'webgpu';
    },
    completeIcon(sample: PrismIconMorphSample | RouteSample) {
      if (activeAction?.kind !== 'icon') throw new Error('No icon transition is active.');
      const normalized = recordSample(sample);
      iconProgress = normalized.progress;
      owner = iconProgress <= EPSILON ? 'svg' : 'webgpu';
      activeAction = null;
    },
    recordSpinnerSample(sample: RouteSample) {
      const normalized = recordSample(sample);
      spinnerProgress = normalized.progress;
      owner = 'webgpu';
    },
    completeSpinner(sample: RouteSample, nextOwner: 'webgpu' | 'atlas') {
      if (activeAction?.kind !== 'spinner') throw new Error('No spinner transition is active.');
      const normalized = recordSample(sample);
      spinnerProgress = normalized.progress;
      owner = nextOwner;
      activeAction = null;
    },
    get snapshot() {
      return {
        activeKind: activeAction?.kind ?? null,
        iconProgress,
        owner,
        reducedMotion,
        rendererReadiness,
        requestedMode,
        spinnerProgress,
        spinnerReadiness,
        ...shared,
      };
    },
  };
}
