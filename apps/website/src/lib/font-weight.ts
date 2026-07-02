export type FontWeightFont = {
  id?: string | null;
  weight: number;
  state: string;
  subfamilyDisplayName?: string | null;
};

export type FontWeightFamily = {
  familyName: string;
  fonts: readonly FontWeightFont[];
};

export type FontWeightLabel = {
  value: number;
  label: string;
};

export type FontWeightItem = {
  value: number;
  label: string;
};

export const activeFontsByWeight = <T extends FontWeightFont>(fonts: readonly T[]): T[] => [
  ...new Map(
    fonts
      .filter((font) => font.state === 'ACTIVE')
      .toSorted((a, b) => a.weight - b.weight)
      .map((font) => [font.weight, font]),
  ).values(),
];

export const matchFontWeight = (weights: readonly number[], target: number): number | undefined => {
  if (weights.length === 0) return undefined;

  const sorted = [...new Set(weights)].toSorted((a, b) => a - b);
  if (target >= 400 && target <= 500) {
    return (
      sorted.find((weight) => weight >= target && weight <= 500) ??
      sorted.findLast((weight) => weight < target) ??
      sorted.find((weight) => weight > 500)
    );
  }

  if (target < 400) {
    return sorted.findLast((weight) => weight <= target) ?? sorted.find((weight) => weight > target);
  }

  return sorted.find((weight) => weight >= target) ?? sorted.findLast((weight) => weight < target);
};

export function resolveFontWeightForFamily(fontFamilies: readonly FontWeightFamily[], familyName: string, currentWeight: number): number;
export function resolveFontWeightForFamily(
  fontFamilies: readonly FontWeightFamily[],
  familyName: string,
  currentWeight: undefined,
): undefined;
export function resolveFontWeightForFamily(
  fontFamilies: readonly FontWeightFamily[],
  familyName: string,
  currentWeight: number | undefined,
): number | undefined;
export function resolveFontWeightForFamily(
  fontFamilies: readonly FontWeightFamily[],
  familyName: string,
  currentWeight: number | undefined,
): number | undefined {
  if (currentWeight === undefined) return undefined;

  const family = fontFamilies.find((candidate) => candidate.familyName === familyName);
  if (!family) return currentWeight;

  return (
    matchFontWeight(
      activeFontsByWeight(family.fonts).map((font) => font.weight),
      currentWeight,
    ) ?? currentWeight
  );
}

export const fontWeightLabel = (font: FontWeightFont, labels: readonly FontWeightLabel[]): string =>
  labels.find(({ value }) => value === font.weight)?.label ??
  (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${font.weight})` : String(font.weight));

export const fontWeightValueLabel = (fonts: readonly FontWeightFont[], labels: readonly FontWeightLabel[], value: number): string => {
  const font = fonts.find((candidate) => candidate.weight === value);
  if (!font) return '(알 수 없는 굵기)';
  return (
    labels.find((label) => label.value === value)?.label ??
    (font.subfamilyDisplayName ? `${font.subfamilyDisplayName} (${value})` : String(value))
  );
};

export const fontWeightItemsForFonts = (fonts: readonly FontWeightFont[], labels: readonly FontWeightLabel[]): FontWeightItem[] =>
  activeFontsByWeight(fonts).map((font) => ({ value: font.weight, label: fontWeightLabel(font, labels) }));
