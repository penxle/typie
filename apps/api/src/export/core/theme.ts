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
