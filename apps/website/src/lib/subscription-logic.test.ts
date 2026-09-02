import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';
import { shouldShowOnboarding, shouldShowTrialReminder, trialDaysLeft, trialReminderLabel, trialStatusLabel } from './subscription-logic';

describe('shouldShowOnboarding', () => {
  const createdAt = '2026-07-23T00:00:00+09:00';
  const now = dayjs('2026-07-23T10:00:00+09:00');

  it('가입 24시간 이내 + 양 플랫폼 플래그 없음이면 표시한다', () => {
    expect(shouldShowOnboarding({ createdAt, preferences: {}, now })).toBe(true);
  });

  it('모바일 완료 플래그가 있으면 표시하지 않는다', () => {
    expect(shouldShowOnboarding({ createdAt, preferences: { mobileOnboardingCompletedAt: '2026-07-23T01:00:00+09:00' }, now })).toBe(false);
  });

  it('웹 완료 플래그가 있으면 표시하지 않는다', () => {
    expect(shouldShowOnboarding({ createdAt, preferences: { webOnboardingCompletedAt: '2026-07-23T01:00:00+09:00' }, now })).toBe(false);
  });

  it('가입 24시간이 지나면 표시하지 않는다', () => {
    expect(shouldShowOnboarding({ createdAt, preferences: {}, now: dayjs('2026-07-24T00:00:01+09:00') })).toBe(false);
  });
});

describe('trialDaysLeft', () => {
  it('만료까지 남은 일수를 내림해 반환한다', () => {
    expect(trialDaysLeft('2026-07-26T12:00:00+09:00', dayjs('2026-07-23T00:00:00+09:00'))).toBe(3);
  });

  it('만료 당일은 0을 반환한다', () => {
    expect(trialDaysLeft('2026-07-23T23:00:00+09:00', dayjs('2026-07-23T01:00:00+09:00'))).toBe(0);
  });

  it('만료가 지나도 음수 대신 0을 반환한다', () => {
    expect(trialDaysLeft('2026-07-22T00:00:00+09:00', dayjs('2026-07-23T00:00:00+09:00'))).toBe(0);
  });
});

describe('trialStatusLabel', () => {
  it('남은 일수를 무료 체험 문구로 만든다', () => {
    expect(trialStatusLabel(3)).toBe('무료 체험 중 · 3일 남음');
  });

  it('종료 당일은 오늘 종료로 표기한다', () => {
    expect(trialStatusLabel(0)).toBe('무료 체험 중 · 오늘 종료');
  });
});

describe('shouldShowTrialReminder', () => {
  it('3일 이하 + 오늘 미노출이면 표시한다', () => {
    expect(shouldShowTrialReminder({ daysLeft: 3, today: '2026-07-23', lastShownDate: '2026-07-22' })).toBe(true);
  });

  it('오늘 이미 노출했으면 표시하지 않는다', () => {
    expect(shouldShowTrialReminder({ daysLeft: 1, today: '2026-07-23', lastShownDate: '2026-07-23' })).toBe(false);
  });

  it('4일 이상 남으면 표시하지 않는다', () => {
    expect(shouldShowTrialReminder({ daysLeft: 4, today: '2026-07-23', lastShownDate: undefined })).toBe(false);
  });

  it('노출 기록이 없으면 표시한다', () => {
    expect(shouldShowTrialReminder({ daysLeft: 0, today: '2026-07-23', lastShownDate: undefined })).toBe(true);
  });
});

describe('trialReminderLabel', () => {
  it('남은 일수 문구', () => {
    expect(trialReminderLabel(3)).toBe('무료 체험이 3일 뒤 끝나요.');
  });

  it('종료 당일 문구', () => {
    expect(trialReminderLabel(0)).toBe('무료 체험이 오늘 끝나요.');
  });
});
