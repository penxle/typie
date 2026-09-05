import themeData from '@typie/assets/theme.json' with { type: 'json' };

const LIGHT_COLORS: Record<string, string> = {
  ...themeData.shared,
  ...themeData.lightShared,
  ...themeData.variants['light-white'],
};

/** 테마 색상 키를 hex 문자열(# 없이)로 변환. 매칭 실패 시 undefined 반환. */
export const resolveColorToHex = (colorKey: string): string | undefined => {
  const hex = LIGHT_COLORS[colorKey];
  return hex ? hex.replace('#', '') : undefined;
};

const CALLOUT_FILL_ALPHA = 8;

export const resolveCalloutFillToHex = (variant: string): string | undefined => {
  const hex = resolveColorToHex(`ui.callout.${variant}`);
  if (!hex) return undefined;
  return [0, 1, 2]
    .map((index) => Number.parseInt(hex.slice(index * 2, index * 2 + 2), 16))
    .map((channel) => Math.round(255 - (255 - channel) * (CALLOUT_FILL_ALPHA / 255)))
    .map((channel) => channel.toString(16).padStart(2, '0'))
    .join('');
};
