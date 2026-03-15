type PlanRules = {
  maxTotalCharacterCount: number;
  maxTotalBlobSize: number;
};

// spell-checker:disable
export type PlanId = keyof typeof PlanId;
export const PlanId = {
  FULL_ACCESS_1MONTH_WITH_BILLING_KEY: 'PL0FL1MBK',
  FULL_ACCESS_1YEAR_WITH_BILLING_KEY: 'PL0FL1YBK',
  FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE: 'PL0FL1MAP',
  FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE: 'PL0FL1YAP',
  FULL_ACCESS_TRIAL: 'PL0FLTRTR',
} as const;
// spell-checker:enable

export const PlanPair = {
  [PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY]: PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY,
  [PlanId.FULL_ACCESS_1YEAR_WITH_BILLING_KEY]: PlanId.FULL_ACCESS_1MONTH_WITH_BILLING_KEY,
  [PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE]: PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE,
  [PlanId.FULL_ACCESS_1YEAR_WITH_IN_APP_PURCHASE]: PlanId.FULL_ACCESS_1MONTH_WITH_IN_APP_PURCHASE,
} as const;

export const defaultPlanRules: PlanRules = {
  maxTotalCharacterCount: 200_000,
  maxTotalBlobSize: 100 * 1000 * 1000,
};

export const SUBSCRIPTION_GRACE_DAYS = 7;
export const TRIAL_DURATION_DAYS = 14;

type DefaultFont = {
  id: string;
  name: string;
  postScriptName: string;
  weight: number;
  path: string;
};

type DefaultFontFamily = {
  id: string;
  displayName: string;
  familyName: string;
  fonts: DefaultFont[];
};

export const defaultValues = {
  fontFamily: 'Pretendard',
  fontSize: 1200,
  fontWeight: 400,
  textColor: 'black',
  backgroundColor: 'none',
  letterSpacing: 0,
  lineHeight: 160,
  textAlign: 'left',
  maxWidth: 600,
  paragraphIndent: 100,
  blockGap: 100,
} as const;

// spell-checker:disable
export const DEFAULT_FONT_FAMILIES: DefaultFontFamily[] = [
  {
    id: '!DEFAULT_PRETENDARD',
    displayName: '프리텐다드',
    familyName: 'Pretendard',
    fonts: [
      { id: '!DEFAULT_PRETENDARD_100', name: 'Pretendard Thin', postScriptName: 'Pretendard-Thin', weight: 100, path: 'Pretendard-Thin' },
      {
        id: '!DEFAULT_PRETENDARD_200',
        name: 'Pretendard ExtraLight',
        postScriptName: 'Pretendard-ExtraLight',
        weight: 200,
        path: 'Pretendard-ExtraLight',
      },
      {
        id: '!DEFAULT_PRETENDARD_300',
        name: 'Pretendard Light',
        postScriptName: 'Pretendard-Light',
        weight: 300,
        path: 'Pretendard-Light',
      },
      {
        id: '!DEFAULT_PRETENDARD_400',
        name: 'Pretendard Regular',
        postScriptName: 'Pretendard-Regular',
        weight: 400,
        path: 'Pretendard-Regular',
      },
      {
        id: '!DEFAULT_PRETENDARD_500',
        name: 'Pretendard Medium',
        postScriptName: 'Pretendard-Medium',
        weight: 500,
        path: 'Pretendard-Medium',
      },
      {
        id: '!DEFAULT_PRETENDARD_600',
        name: 'Pretendard SemiBold',
        postScriptName: 'Pretendard-SemiBold',
        weight: 600,
        path: 'Pretendard-SemiBold',
      },
      { id: '!DEFAULT_PRETENDARD_700', name: 'Pretendard Bold', postScriptName: 'Pretendard-Bold', weight: 700, path: 'Pretendard-Bold' },
      {
        id: '!DEFAULT_PRETENDARD_800',
        name: 'Pretendard ExtraBold',
        postScriptName: 'Pretendard-ExtraBold',
        weight: 800,
        path: 'Pretendard-ExtraBold',
      },
      {
        id: '!DEFAULT_PRETENDARD_900',
        name: 'Pretendard Black',
        postScriptName: 'Pretendard-Black',
        weight: 900,
        path: 'Pretendard-Black',
      },
    ],
  },
  {
    id: '!DEFAULT_KOPUBWORLDDOTUM',
    displayName: '코펍월드돋움',
    familyName: 'KoPubWorldDotum',
    fonts: [
      {
        id: '!DEFAULT_KOPUBWORLDDOTUM_300',
        name: 'KoPubWorldDotum Light',
        postScriptName: 'KoPubWorldDotumLight',
        weight: 300,
        path: 'KoPubWorldDotum-Light',
      },
      {
        id: '!DEFAULT_KOPUBWORLDDOTUM_500',
        name: 'KoPubWorldDotum Medium',
        postScriptName: 'KoPubWorldDotumMedium',
        weight: 500,
        path: 'KoPubWorldDotum-Medium',
      },
      {
        id: '!DEFAULT_KOPUBWORLDDOTUM_700',
        name: 'KoPubWorldDotum Bold',
        postScriptName: 'KoPubWorldDotumBold',
        weight: 700,
        path: 'KoPubWorldDotum-Bold',
      },
    ],
  },
  {
    id: '!DEFAULT_NANUMBARUNGOTHIC',
    displayName: '나눔바른고딕',
    familyName: 'NanumBarunGothic',
    fonts: [
      {
        id: '!DEFAULT_NANUMBARUNGOTHIC_200',
        name: 'NanumBarunGothic UltraLight',
        postScriptName: 'NanumBarunGothicUltraLight',
        weight: 200,
        path: 'NanumBarunGothic-UltraLight',
      },
      {
        id: '!DEFAULT_NANUMBARUNGOTHIC_300',
        name: 'NanumBarunGothic Light',
        postScriptName: 'NanumBarunGothicLight',
        weight: 300,
        path: 'NanumBarunGothic-Light',
      },
      {
        id: '!DEFAULT_NANUMBARUNGOTHIC_400',
        name: 'NanumBarunGothic',
        postScriptName: 'NanumBarunGothic',
        weight: 400,
        path: 'NanumBarunGothic-Regular',
      },
      {
        id: '!DEFAULT_NANUMBARUNGOTHIC_700',
        name: 'NanumBarunGothic Bold',
        postScriptName: 'NanumBarunGothicBold',
        weight: 700,
        path: 'NanumBarunGothic-Bold',
      },
    ],
  },
  {
    id: '!DEFAULT_RIDIBATANG',
    displayName: '리디바탕',
    familyName: 'RIDIBatang',
    fonts: [{ id: '!DEFAULT_RIDIBATANG_400', name: 'RIDIBatang', postScriptName: 'RIDIBatang', weight: 400, path: 'RIDIBatang-Regular' }],
  },
  {
    id: '!DEFAULT_KOPUBWORLDBATANG',
    displayName: '코펍월드바탕',
    familyName: 'KoPubWorldBatang',
    fonts: [
      {
        id: '!DEFAULT_KOPUBWORLDBATANG_300',
        name: 'KoPubWorldBatang Light',
        postScriptName: 'KoPubWorldBatangLight',
        weight: 300,
        path: 'KoPubWorldBatang-Light',
      },
      {
        id: '!DEFAULT_KOPUBWORLDBATANG_500',
        name: 'KoPubWorldBatang Medium',
        postScriptName: 'KoPubWorldBatangMedium',
        weight: 500,
        path: 'KoPubWorldBatang-Medium',
      },
      {
        id: '!DEFAULT_KOPUBWORLDBATANG_700',
        name: 'KoPubWorldBatang Bold',
        postScriptName: 'KoPubWorldBatangBold',
        weight: 700,
        path: 'KoPubWorldBatang-Bold',
      },
    ],
  },
  {
    id: '!DEFAULT_NANUMMYEONGJO',
    displayName: '나눔명조',
    familyName: 'NanumMyeongjo',
    fonts: [
      {
        id: '!DEFAULT_NANUMMYEONGJO_400',
        name: 'NanumMyeongjo',
        postScriptName: 'NanumMyeongjo',
        weight: 400,
        path: 'NanumMyeongjo-Regular',
      },
      {
        id: '!DEFAULT_NANUMMYEONGJO_700',
        name: 'NanumMyeongjoBold',
        postScriptName: 'NanumMyeongjoBold',
        weight: 700,
        path: 'NanumMyeongjo-Bold',
      },
      {
        id: '!DEFAULT_NANUMMYEONGJO_800',
        name: 'NanumMyeongjoExtraBold',
        postScriptName: 'NanumMyeongjoExtraBold',
        weight: 800,
        path: 'NanumMyeongjo-ExtraBold',
      },
    ],
  },
];
// spell-checker:enable
