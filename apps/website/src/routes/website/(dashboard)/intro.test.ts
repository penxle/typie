import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';
import { MARKETING_CONSENT_MIN_CHARACTERS, selectIntro, TRIAL_EXPIRED_SURVEY_NAME } from './intro.ts';
import { USER_SURVEY_NAME } from './user-survey.ts';
import type { IntroInput } from './intro.ts';

const now = dayjs('2026-09-02T10:00:00+09:00');

const base: IntroInput = {
  now,
  deepLink: false,
  createdAt: '2026-08-01T00:00:00+09:00',
  preferences: {},
  surveys: [],
  marketingConsentAskedAt: '2026-08-02T00:00:00+09:00',
  totalCharacterCount: 0,
  userSurveySnoozedUntil: null,
};

const newUser = { createdAt: '2026-09-02T00:00:00+09:00' };

describe('selectIntro', () => {
  it('자격자가 없으면 null 을 돌려준다', () => {
    expect(selectIntro(base)).toBeNull();
  });

  it('가입 24시간 이내 미완료면 온보딩', () => {
    expect(selectIntro({ ...base, ...newUser })).toBe('onboarding');
  });

  it('온보딩을 마친 신규 가입자는 온보딩을 다시 보지 않는다', () => {
    expect(selectIntro({ ...base, ...newUser, preferences: { webOnboardingCompletedAt: '2026-09-02T01:00:00+09:00' } })).toBeNull();
  });

  it('체험종료 설문 자격이면 체험종료', () => {
    expect(selectIntro({ ...base, surveys: [TRIAL_EXPIRED_SURVEY_NAME] })).toBe('trial_expired');
  });

  it('이용자 설문 자격이고 스누즈가 없으면 설문', () => {
    expect(selectIntro({ ...base, surveys: [USER_SURVEY_NAME] })).toBe('user_survey');
  });

  it('이용자 설문이 스누즈 중이면 설문을 건너뛴다', () => {
    expect(selectIntro({ ...base, surveys: [USER_SURVEY_NAME], userSurveySnoozedUntil: '2026-09-10T00:00:00+09:00' })).toBeNull();
  });

  it('스누즈가 지났으면 설문을 다시 띄운다', () => {
    expect(selectIntro({ ...base, surveys: [USER_SURVEY_NAME], userSurveySnoozedUntil: '2026-09-01T00:00:00+09:00' })).toBe('user_survey');
  });

  it('마케팅 동의를 묻지 않았고 글자 수가 기준 이상이면 마케팅 동의', () => {
    expect(selectIntro({ ...base, marketingConsentAskedAt: null, totalCharacterCount: MARKETING_CONSENT_MIN_CHARACTERS })).toBe(
      'marketing_consent',
    );
  });

  it('글자 수가 기준 미만이면 마케팅 동의를 묻지 않는다', () => {
    expect(selectIntro({ ...base, marketingConsentAskedAt: null, totalCharacterCount: MARKETING_CONSENT_MIN_CHARACTERS - 1 })).toBeNull();
  });

  it('이미 마케팅 동의를 물었으면 글자 수와 무관하게 묻지 않는다', () => {
    expect(selectIntro({ ...base, totalCharacterCount: 10_000 })).toBeNull();
  });

  it('딥링크가 있으면 온보딩 자격이 있어도 null', () => {
    expect(selectIntro({ ...base, ...newUser, deepLink: true })).toBeNull();
  });

  it('온보딩과 체험종료가 동시에 자격이면 온보딩', () => {
    expect(selectIntro({ ...base, ...newUser, surveys: [TRIAL_EXPIRED_SURVEY_NAME] })).toBe('onboarding');
  });

  it('체험종료와 설문이 동시에 자격이면 체험종료', () => {
    expect(selectIntro({ ...base, surveys: [USER_SURVEY_NAME, TRIAL_EXPIRED_SURVEY_NAME] })).toBe('trial_expired');
  });

  it('설문과 마케팅 동의가 동시에 자격이면 설문', () => {
    expect(selectIntro({ ...base, surveys: [USER_SURVEY_NAME], marketingConsentAskedAt: null, totalCharacterCount: 500 })).toBe(
      'user_survey',
    );
  });

  it('설문이 스누즈 중이면 마케팅 동의로 넘어간다', () => {
    expect(
      selectIntro({
        ...base,
        surveys: [USER_SURVEY_NAME],
        userSurveySnoozedUntil: '2026-09-10T00:00:00+09:00',
        marketingConsentAskedAt: null,
        totalCharacterCount: 500,
      }),
    ).toBe('marketing_consent');
  });
});
