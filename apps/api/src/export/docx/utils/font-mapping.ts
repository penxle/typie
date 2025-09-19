// cspell:disable
export const defaultFontMapping: Record<string, string> = {
  Pretendard: 'Pretendard',
  KoPubWorldDotum: 'KoPubWorld돋움체_Pro',
  KoPubWorldBatang: 'KoPubWorld바탕체_Pro',
  NanumBarunGothic: 'NanumBarunGothicOTF',
  NanumMyeongjo: 'Nanum Myeongjo',
  RIDIBatang: 'RIDIBatang',
};

export const fontWeightMapping: Record<string, Record<number, string>> = {
  Pretendard: {
    100: 'Pretendard Thin',
    200: 'Pretendard ExtraLight',
    300: 'Pretendard Light',
    400: 'Pretendard Regular',
    500: 'Pretendard Medium',
    600: 'Pretendard SemiBold',
    700: 'Pretendard Bold',
    800: 'Pretendard ExtraBold',
    900: 'Pretendard Black',
  },
  KoPubWorldDotum: {
    500: 'KoPubWorld돋움체_Pro Medium',
    700: 'KoPubWorld돋움체_Pro Bold',
  },
  KoPubWorldBatang: {
    500: 'KoPubWorld바탕체_Pro Medium',
    700: 'KoPubWorld바탕체_Pro Bold',
  },
  NanumBarunGothic: {
    400: 'NanumBarunGothicOTF Regular',
    700: 'NanumBarunGothicOTF Bold',
  },
  NanumMyeongjo: {
    400: 'Nanum Myeongjo Regular',
    700: 'Nanum Myeongjo Bold',
  },
  RIDIBatang: {
    400: 'RIDIBatang Regular',
  },
};
// cspell:enable

export type CustomFont = {
  id: string;
  familyName?: string | null;
  fullName?: string | null;
  postScriptName?: string | null;
  weight?: number;
};

export class FontMapper {
  private customFonts = new Map<string, CustomFont>();
  private fontsByWeight = new Map<string, Map<number, CustomFont>>();

  addCustomFont(font: CustomFont) {
    this.customFonts.set(font.id, font);

    if (font.id.startsWith('FNTF') && font.weight !== undefined) {
      if (!this.fontsByWeight.has(font.id)) {
        this.fontsByWeight.set(font.id, new Map());
      }
      const weightMap = this.fontsByWeight.get(font.id);
      if (weightMap) {
        weightMap.set(font.weight, font);
      }
    }
  }

  getFontName(fontId: string | undefined, weight?: number): string | undefined {
    if (!fontId) return undefined;

    if (defaultFontMapping[fontId]) {
      if (weight && fontWeightMapping[fontId]) {
        if (fontWeightMapping[fontId][weight]) {
          const fontName = fontWeightMapping[fontId][weight];
          return fontName;
        }

        const supportedWeights = Object.keys(fontWeightMapping[fontId])
          .map(Number)
          .toSorted((a, b) => a - b);
        if (supportedWeights.length > 0) {
          const closestWeight = weight >= 700 ? (supportedWeights.at(-1) ?? supportedWeights[0]) : supportedWeights[0];

          const fontName = fontWeightMapping[fontId][closestWeight];
          return fontName;
        }
      }

      const fontName = defaultFontMapping[fontId];
      return fontName;
    }

    if (fontId.startsWith('FNTF')) {
      const weightMap = this.fontsByWeight.get(fontId);
      if (weightMap) {
        const targetWeight = weight || 400;

        const exactFont = weightMap.get(targetWeight);
        if (exactFont) {
          // NOTE: 폰트 선택 전략:
          // 1. weight가 400이 아닌 경우: fullName 우선
          // 2. 한글 familyName이 있고 fullName이 한글이 아닌 경우: postScriptName 우선
          // 3. 그 외: fullName 우선
          let fontName: string;

          const hasKoreanName =
            (exactFont.familyName && /[가-힣]/.test(exactFont.familyName)) || (exactFont.fullName && /[가-힣]/.test(exactFont.fullName));
          const hasEnglishPostScript = exactFont.postScriptName && !/[가-힣]/.test(exactFont.postScriptName);

          if (targetWeight && targetWeight !== 400) {
            fontName = exactFont.fullName || exactFont.familyName || exactFont.postScriptName || fontId;
          } else if (hasKoreanName && hasEnglishPostScript && exactFont.postScriptName) {
            fontName = exactFont.postScriptName;
          } else {
            fontName = exactFont.fullName || exactFont.familyName || exactFont.postScriptName || fontId;
          }
          return fontName;
        }

        const weights = [...weightMap.keys()].toSorted((a, b) => a - b);
        const closestWeight = weights.reduce((prev, curr) => {
          return Math.abs(curr - targetWeight) < Math.abs(prev - targetWeight) ? curr : prev;
        });
        const closestFont = weightMap.get(closestWeight);
        if (closestFont) {
          const fontName =
            closestWeight === 400
              ? closestFont.familyName || closestFont.fullName || closestFont.postScriptName || fontId
              : closestFont.fullName || closestFont.familyName || closestFont.postScriptName || fontId;
          return fontName;
        }
      }
    }

    const customFont = this.customFonts.get(fontId);
    if (customFont) {
      const fontName = customFont.fullName || customFont.familyName || customFont.postScriptName || fontId;
      return fontName;
    }

    return defaultFontMapping[fontId] || fontId;
  }
}

export function extractFontIds(content: Record<string, unknown>): Set<string> {
  const fontIds = new Set<string>();

  function traverse(node: Record<string, unknown>) {
    if (!node) return;

    if (node.marks && Array.isArray(node.marks)) {
      for (const mark of node.marks) {
        if (mark.type === 'text_style' && mark.attrs?.fontFamily) {
          fontIds.add(mark.attrs.fontFamily);
        }
      }
    }

    if (node.content && Array.isArray(node.content)) {
      for (const child of node.content) {
        traverse(child);
      }
    }
  }

  traverse(content);
  return fontIds;
}
