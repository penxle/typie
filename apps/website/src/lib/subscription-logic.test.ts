import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';
import {
  isLegacyTrial,
  shouldShowOnboarding,
  shouldShowTrialReminder,
  trialDaysLeft,
  trialReminderLabel,
  trialStatusLabel,
} from './subscription-logic';

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
  it('일반 트라이얼은 무료 체험 문구를 쓴다', () => {
    expect(trialStatusLabel(3, false)).toBe('무료 체험 중 · 3일 남음');
  });

  it('종료 당일은 오늘 종료로 표기한다', () => {
    expect(trialStatusLabel(0, false)).toBe('무료 체험 중 · 오늘 종료');
  });

  it('레거시 트라이얼은 무료 이용 기간 문구를 쓴다', () => {
    expect(trialStatusLabel(2, true)).toBe('무료 이용 기간 · 2일 남음');
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
  it('일반 트라이얼 문구', () => {
    expect(trialReminderLabel(3, false)).toBe('무료 체험이 3일 뒤 끝나요.');
  });

  it('종료 당일 문구', () => {
    expect(trialReminderLabel(0, false)).toBe('무료 체험이 오늘 끝나요.');
  });

  it('레거시 문구', () => {
    expect(trialReminderLabel(1, true)).toBe('무료 이용 기간이 1일 뒤 끝나요.');
  });
});

describe('isLegacyTrial', () => {
  it('컷오프 이전 시작 트라이얼은 레거시다', () => {
    expect(isLegacyTrial({ availability: 'TRIAL', startsAt: '2026-07-12T00:00:00+09:00' })).toBe(true);
  });

  it('컷오프 시각 정각도 레거시다', () => {
    expect(isLegacyTrial({ availability: 'TRIAL', startsAt: '2026-07-13T00:00:00+09:00' })).toBe(true);
  });

  it('컷오프 이후 시작은 레거시가 아니다', () => {
    expect(isLegacyTrial({ availability: 'TRIAL', startsAt: '2026-07-13T00:00:01+09:00' })).toBe(false);
  });

  it('트라이얼이 아니면 레거시가 아니다', () => {
    expect(isLegacyTrial({ availability: 'BILLING_KEY', startsAt: '2026-07-01T00:00:00+09:00' })).toBe(false);
  });
});
