import type { ExportFontFamily } from './types';

/** weight 배열에서 target에 가장 가까운 항목을 찾는다 */
export function nearestWeight<T extends { weight: number }>(weights: T[], target: number): T | undefined {
  if (weights.length === 0) return undefined;
  let best = weights[0];
  let bestDist = Math.abs(best.weight - target);
  for (let i = 1; i < weights.length; i++) {
    const dist = Math.abs(weights[i].weight - target);
    if (dist < bestDist || (dist === bestDist && weights[i].weight > best.weight)) {
      best = weights[i];
      bestDist = dist;
    }
  }
  return best;
}

/** fonts 배열에서 family name으로 ExportFontFamily를 찾는다 */
export function findFontFamily(fonts: ExportFontFamily[], family: string): ExportFontFamily | undefined {
  return fonts.find((f) => f.family === family);
}
