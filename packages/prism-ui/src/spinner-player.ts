import { PRISM_SPINNER_HDR_HEADROOM_DEFAULT } from './internal/prism-spinner-hdr.ts';
import { PRISM_SPINNER_CSS_SIZE } from './internal/prism-spinner-morph.ts';
import type { PrismSpinnerAtlasConfiguration, PrismSpinnerPreRenderedHdrConfiguration } from './internal/prism-spinner-prerendered.ts';

export const PRISM_SPINNER_DURATION_MS = 2400;
export const PRISM_SPINNER_FRAME_COUNT = 144;
export const PRISM_SPINNER_ATLAS_DURATION_MS = 2300;
export const PRISM_SPINNER_ATLAS_FRAME_COUNT = 138;
const PRISM_SPINNER_ATLAS_COLUMNS = 12;
const PRISM_SPINNER_ATLAS_ROWS = 12;

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
