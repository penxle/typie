export const DPI = 96;

export function pxToCm(px: number, dpi = DPI): number {
  return (px / dpi) * 2.54;
}

export function cmToPx(cm: number, dpi = DPI): number {
  return (cm * dpi) / 2.54;
}

export type Tick = {
  logicalPosition: number;
  position: number;
  isMajor: boolean;
  label?: number;
};

type CalculateTicksOptions = {
  totalSize: number;
  unit: 'px' | 'cm';
  dpi: number;
  zoom?: number;
};

const MIN_MINOR_DISPLAY_PX = 5;
const TARGET_MAJOR_DISPLAY_PX = 80;

function pickNearest(value: number, candidates: readonly number[]): number {
  let best = candidates[0] ?? value;
  let bestScore = Infinity;

  for (const candidate of candidates) {
    const score = Math.abs(candidate - value);
    if (score < bestScore) {
      best = candidate;
      bestScore = score;
    }
  }

  return best;
}

function pickMinorDivisor(majorDisplayPx: number): number {
  const divisors = [10, 8, 5, 4, 2, 1] as const;
  for (const divisor of divisors) {
    if (majorDisplayPx / divisor >= MIN_MINOR_DISPLAY_PX) {
      return divisor;
    }
  }
  return 1;
}

export function calculateTicks({ totalSize, unit, dpi, zoom = 1 }: CalculateTicksOptions): Tick[] {
  const ticks: Tick[] = [];
  const safeZoom = Number.isFinite(zoom) && zoom > 0 ? zoom : 1;

  if (unit === 'px') {
    const majorCandidates = [10, 20, 50, 100, 200, 500, 1000, 2000] as const;
    const majorInterval = pickNearest(TARGET_MAJOR_DISPLAY_PX / safeZoom, majorCandidates);
    const majorDisplayPx = majorInterval * safeZoom;
    const minorDivisor = pickMinorDivisor(majorDisplayPx);
    const minorInterval = majorInterval / minorDivisor;
    const tickCount = Math.floor(totalSize / minorInterval);

    for (let i = 0; i <= tickCount; i++) {
      const logicalPosition = i * minorInterval;
      const isMajor = i % minorDivisor === 0;
      ticks.push({
        logicalPosition,
        position: logicalPosition * safeZoom,
        isMajor,
        label: isMajor ? logicalPosition : undefined,
      });
    }
  } else if (unit === 'cm') {
    const majorCandidatesCm = [0.1, 0.2, 0.25, 0.5, 1, 2, 5, 10, 20] as const;
    const targetMajorCm = pxToCm(TARGET_MAJOR_DISPLAY_PX / safeZoom, dpi);
    const majorIntervalCm = pickNearest(targetMajorCm, majorCandidatesCm);
    const majorDisplayPx = cmToPx(majorIntervalCm, dpi) * safeZoom;
    const minorDivisor = pickMinorDivisor(majorDisplayPx);
    const minorIntervalCm = majorIntervalCm / minorDivisor;
    const totalSizeCm = pxToCm(totalSize, dpi);
    const tickCount = Math.floor(totalSizeCm / minorIntervalCm);

    for (let i = 0; i <= tickCount; i++) {
      const cm = i * minorIntervalCm;
      const isMajor = i % minorDivisor === 0;
      const logicalPosition = cmToPx(cm, dpi);

      ticks.push({
        logicalPosition,
        position: logicalPosition * safeZoom,
        isMajor,
        label: isMajor ? cm : undefined,
      });
    }
  }

  return ticks;
}

export function formatTickLabel(value: number, unit: 'px' | 'cm'): string {
  if (unit === 'px') {
    return Math.round(value).toString();
  }
  const rounded = Math.round(value * 100) / 100;
  if (Math.abs(rounded - Math.round(rounded)) < 0.0001) {
    return Math.round(rounded).toString();
  }
  if (Math.abs(rounded * 10 - Math.round(rounded * 10)) < 0.0001) {
    return rounded.toFixed(1);
  }
  return rounded.toFixed(2);
}
