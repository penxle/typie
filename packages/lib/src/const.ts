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
  LIFETIME_ACCESS: 'PL0LIFETIME',
} as const;
// spell-checker:enable

// spell-checker:disable
export type PromptId = keyof typeof PromptId;
export const PromptId = {
  SUMMARIZE: 'PRMT0SUMMARIZE',
  ANALYZE: 'PRMT0ANALYZE',
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

export const APP_STORE_URL = 'https://apps.apple.com/app/id6745595771';
export const PLAY_STORE_URL = 'https://play.google.com/store/apps/details?id=co.typie';

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
