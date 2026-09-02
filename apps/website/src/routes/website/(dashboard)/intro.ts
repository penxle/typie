import { shouldShowOnboarding } from '$lib/subscription-logic';
import { isUserSurveySnoozed, USER_SURVEY_NAME } from './user-survey';
import type { Dayjs } from 'dayjs';

export const TRIAL_EXPIRED_SURVEY_NAME = 'trial_expired_modal';
export const MARKETING_CONSENT_MIN_CHARACTERS = 100;

export type IntroKind = 'onboarding' | 'trial_expired' | 'user_survey' | 'marketing_consent';

export type IntroInput = {
  now: Dayjs;
  deepLink: boolean;
  createdAt: string;
  preferences: Record<string, unknown>;
  surveys: readonly string[];
  marketingConsentAskedAt: string | null;
  totalCharacterCount: number;
  userSurveySnoozedUntil: string | null;
};

export const selectIntro = (input: IntroInput): IntroKind | null => {
  if (input.deepLink) {
    return null;
  }

  if (shouldShowOnboarding({ createdAt: input.createdAt, preferences: input.preferences, now: input.now })) {
    return 'onboarding';
  }

  if (input.surveys.includes(TRIAL_EXPIRED_SURVEY_NAME)) {
    return 'trial_expired';
  }

  if (input.surveys.includes(USER_SURVEY_NAME) && !isUserSurveySnoozed(input.userSurveySnoozedUntil, input.now.toDate())) {
    return 'user_survey';
  }

  if (input.marketingConsentAskedAt === null && input.totalCharacterCount >= MARKETING_CONSENT_MIN_CHARACTERS) {
    return 'marketing_consent';
  }

  return null;
};
