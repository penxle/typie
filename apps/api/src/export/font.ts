export type FontNameEntry = {
  weight: number;
  postScriptName: string;
  faceName: string;
  faceDefault: string;
};

export type FontNameMap = Map<string, FontNameEntry[]>;

/** fontNameMap에서 familyName + weight에 가장 가까운 엔트리를 찾는다 */
export function resolveFontEntry(map: FontNameMap, familyName: string, weight: number): FontNameEntry | undefined {
  const entries = map.get(familyName);
  if (!entries || entries.length === 0) return undefined;
  let best = entries[0];
  let bestDist = Math.abs(best.weight - weight);
  for (let i = 1; i < entries.length; i++) {
    const dist = Math.abs(entries[i].weight - weight);
    if (dist < bestDist) {
      best = entries[i];
      bestDist = dist;
    }
  }
  return best;
}
