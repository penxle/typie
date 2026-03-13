import type { ExportFontFamily } from './types';

/** CSS Fonts Level 4 §5.2 font-weight matching algorithm */
export function nearestWeight<T extends { weight: number }>(weights: T[], target: number): T | undefined {
  if (weights.length === 0) return undefined;

  const sorted = weights.toSorted((a, b) => a.weight - b.weight);

  if (target >= 400 && target <= 500) {
    const ascending = sorted.find((w) => w.weight >= target && w.weight <= 500);
    if (ascending) return ascending;
    const descending = sorted.findLast((w) => w.weight < target);
    if (descending) return descending;
    const above500 = sorted.find((w) => w.weight > 500);
    if (above500) return above500;
  } else if (target < 400) {
    const descending = sorted.findLast((w) => w.weight <= target);
    if (descending) return descending;
    const ascending = sorted.find((w) => w.weight > target);
    if (ascending) return ascending;
  } else {
    const ascending = sorted.find((w) => w.weight >= target);
    if (ascending) return ascending;
    const descending = sorted.findLast((w) => w.weight < target);
    if (descending) return descending;
  }

  return sorted[0];
}

/** fonts 배열에서 family name으로 ExportFontFamily를 찾는다 */
export function findFontFamily(fonts: ExportFontFamily[], family: string): ExportFontFamily | undefined {
  return fonts.find((f) => f.family === family);
}
