import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

export const NEW_USER_WINDOW_HOURS = 24;
export const TRIAL_REMINDER_THRESHOLD_DAYS = 3;

type OnboardingParams = {
  createdAt: string;
  preferences: Record<string, unknown>;
  now: Dayjs;
};

export const shouldShowOnboarding = ({ createdAt, preferences, now }: OnboardingParams): boolean => {
  if (now.diff(dayjs(createdAt), 'hour') >= NEW_USER_WINDOW_HOURS) {
    return false;
  }

  return !preferences.mobileOnboardingCompletedAt && !preferences.webOnboardingCompletedAt;
};

export const trialDaysLeft = (endsAt: string, now: Dayjs): number => {
  return Math.max(0, dayjs(endsAt).diff(now, 'day'));
};

// LIFETIME·MANUAL은 주기 종료를 sentinel(9999-12-31)로 저장한다. 값은 유지하되 날짜로 렌더하지 않는다.
export const INDEFINITE_PERIOD_YEAR = 9999;

export const isIndefinitePeriod = (endsAt: string): boolean => {
  return dayjs(endsAt).year() >= INDEFINITE_PERIOD_YEAR;
};

export const trialStatusLabel = (daysLeft: number): string => {
  return daysLeft <= 0 ? '무료 체험 중 · 오늘 종료' : `무료 체험 중 · ${daysLeft}일 남음`;
};

type ReminderParams = {
  daysLeft: number;
  today: string;
  lastShownDate?: string;
};

export const shouldShowTrialReminder = ({ daysLeft, today, lastShownDate }: ReminderParams): boolean => {
  return daysLeft <= TRIAL_REMINDER_THRESHOLD_DAYS && lastShownDate !== today;
};

export const trialReminderLabel = (daysLeft: number): string => {
  const when = daysLeft <= 0 ? '오늘' : `${daysLeft}일 뒤`;
  return `무료 체험이 ${when} 끝나요.`;
};
