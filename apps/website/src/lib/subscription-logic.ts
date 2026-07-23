import dayjs from 'dayjs';
import type { Dayjs } from 'dayjs';

export const NEW_USER_WINDOW_HOURS = 24;
export const TRIAL_REMINDER_THRESHOLD_DAYS = 3;
export const LEGACY_TRIAL_CUTOFF = '2026-07-13T00:00:00+09:00';

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

export const trialDaysLeft = (expiresAt: string, now: Dayjs): number => {
  return Math.max(0, dayjs(expiresAt).diff(now, 'day'));
};

export const trialStatusLabel = (daysLeft: number, legacy: boolean): string => {
  const prefix = legacy ? '무료 이용 기간' : '무료 체험 중';
  return daysLeft <= 0 ? `${prefix} · 오늘 종료` : `${prefix} · ${daysLeft}일 남음`;
};

type ReminderParams = {
  daysLeft: number;
  today: string;
  lastShownDate?: string;
};

export const shouldShowTrialReminder = ({ daysLeft, today, lastShownDate }: ReminderParams): boolean => {
  return daysLeft <= TRIAL_REMINDER_THRESHOLD_DAYS && lastShownDate !== today;
};

export const trialReminderLabel = (daysLeft: number, legacy: boolean): string => {
  const subject = legacy ? '무료 이용 기간이' : '무료 체험이';
  const when = daysLeft <= 0 ? '오늘' : `${daysLeft}일 뒤`;
  return `${subject} ${when} 끝나요.`;
};

type LegacyTrialParams = {
  availability: string;
  startsAt: string;
};

export const isLegacyTrial = ({ availability, startsAt }: LegacyTrialParams): boolean => {
  return availability === 'TRIAL' && !dayjs(startsAt).isAfter(dayjs(LEGACY_TRIAL_CUTOFF));
};
