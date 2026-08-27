import { PRISM_SPINNER_HDR_HEADROOM_DEFAULT } from './internal/prism-spinner-hdr.ts';
import { PRISM_SPINNER_CSS_SIZE, spinnerPhaseForFrameIndex } from './internal/prism-spinner-morph.ts';
import type { PrismSpinnerAtlasConfiguration, PrismSpinnerPreRenderedHdrConfiguration } from './internal/prism-spinner-prerendered.ts';

export const PRISM_SPINNER_DURATION_MS = 2400;
export const PRISM_SPINNER_FRAME_COUNT = 144;
export const PRISM_SPINNER_ATLAS_DURATION_MS = 2300;
export const PRISM_SPINNER_ATLAS_FRAME_COUNT = 138;
const PRISM_SPINNER_ATLAS_COLUMNS = 12;
const PRISM_SPINNER_ATLAS_ROWS = 12;
const PRISM_SPINNER_DURATION_SECONDS = PRISM_SPINNER_DURATION_MS / 1000;
const PRISM_SPINNER_MINIMUM_VELOCITY = 0.15;
const PRISM_SPINNER_VELOCITY_AMPLITUDE =
  (2 * (1 - PRISM_SPINNER_MINIMUM_VELOCITY * PRISM_SPINNER_DURATION_SECONDS)) / PRISM_SPINNER_DURATION_SECONDS;

type PrismSpinnerAssetSelection = {
  atlas: PrismSpinnerAtlasConfiguration;
  dpr: 1 | 2 | 3;
  hdr: PrismSpinnerPreRenderedHdrConfiguration;
  sdrUrl: string;
};

function nearestDpr(value: number): 1 | 2 | 3 {
  const normalized = Number.isFinite(value) ? value : 1;
  if (normalized < 1.5) return 1;
  if (normalized < 2.5) return 2;
  return 3;
}

export function samplePrismSpinnerPlayback(elapsedMs: number): {
  complete: boolean;
  frameIndex: number;
  phase: number;
  turns: number;
  turnsPerSecond: number;
} {
  if (!Number.isFinite(elapsedMs) || elapsedMs < 0) throw new RangeError('Spinner elapsed time must be finite and non-negative.');
  const elapsedSeconds = Math.min(elapsedMs, PRISM_SPINNER_DURATION_MS) / 1000;
  const completedLoops = Math.floor(elapsedSeconds / PRISM_SPINNER_DURATION_SECONDS);
  const localSeconds = elapsedSeconds - completedLoops * PRISM_SPINNER_DURATION_SECONDS;
  const integratedWave =
    localSeconds / 2 -
    (PRISM_SPINNER_DURATION_SECONDS * Math.sin((2 * Math.PI * localSeconds) / PRISM_SPINNER_DURATION_SECONDS)) / (4 * Math.PI);
  const turns = completedLoops + PRISM_SPINNER_MINIMUM_VELOCITY * localSeconds + PRISM_SPINNER_VELOCITY_AMPLITUDE * integratedWave;
  const phase = turns - Math.floor(turns);
  const frameIndex = Math.round(phase * PRISM_SPINNER_ATLAS_FRAME_COUNT) % PRISM_SPINNER_ATLAS_FRAME_COUNT;
  return {
    complete: elapsedMs >= PRISM_SPINNER_DURATION_MS,
    frameIndex,
    phase: spinnerPhaseForFrameIndex(frameIndex, PRISM_SPINNER_ATLAS_FRAME_COUNT),
    turns,
    turnsPerSecond:
      PRISM_SPINNER_MINIMUM_VELOCITY +
      PRISM_SPINNER_VELOCITY_AMPLITUDE * Math.sin((Math.PI * localSeconds) / PRISM_SPINNER_DURATION_SECONDS) ** 2,
  };
}

export function resolvePrismSpinnerAssets(devicePixelRatio = globalThis.devicePixelRatio || 1): PrismSpinnerAssetSelection {
  const dpr = nearestDpr(devicePixelRatio);
  const hdr: PrismSpinnerPreRenderedHdrConfiguration = {
    assetUrl: new URL(`../assets/spinner/prism-spinner-hdr@${dpr}x.bin`, import.meta.url).href,
    cssSize: PRISM_SPINNER_CSS_SIZE,
    durationMs: PRISM_SPINNER_DURATION_MS,
    frameCount: PRISM_SPINNER_FRAME_COUNT,
    headroom: PRISM_SPINNER_HDR_HEADROOM_DEFAULT,
    mode: 'auto',
  };
  return {
    atlas: {
      atlasUrl: new URL(`../assets/spinner/prism-spinner-atlas-sdr@${dpr}x.png`, import.meta.url).href,
      columns: PRISM_SPINNER_ATLAS_COLUMNS,
      cssSize: PRISM_SPINNER_CSS_SIZE,
      durationMs: PRISM_SPINNER_ATLAS_DURATION_MS,
      frameCount: PRISM_SPINNER_ATLAS_FRAME_COUNT,
      framePixelSize: PRISM_SPINNER_CSS_SIZE * dpr,
      hdr: {
        ...hdr,
        assetUrl: new URL(`../assets/spinner/prism-spinner-atlas-hdr@${dpr}x.bin`, import.meta.url).href,
        durationMs: PRISM_SPINNER_ATLAS_DURATION_MS,
        frameCount: PRISM_SPINNER_ATLAS_FRAME_COUNT,
      },
      rows: PRISM_SPINNER_ATLAS_ROWS,
    },
    dpr,
    hdr,
    sdrUrl: new URL(`../assets/spinner/prism-spinner-sdr@${dpr}x.apng`, import.meta.url).href,
  };
}

export function resolvePrismSpinnerApngSource(devicePixelRatio = globalThis.devicePixelRatio || 1, animationRun = 1): string {
  const source = new URL(resolvePrismSpinnerAssets(devicePixelRatio).sdrUrl);
  source.searchParams.set('animation-run', String(animationRun));
  return source.href;
}
