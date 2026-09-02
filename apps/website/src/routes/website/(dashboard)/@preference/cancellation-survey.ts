export const CANCELLATION_SURVEY_NAME = 'subscription_cancellation_202609';

export type CancellationReason =
  'billing_change' | 'not_now' | 'low_usage' | 'expensive' | 'unstable' | 'lack_features' | 'switched' | 'other' | 'prefer_not_to_say';

export type CancellationGuidance = 'payment_method' | 'waiver' | 'support' | 'feature_request';

export type CancellationReasonOption = {
  value: CancellationReason;
  label: string;
  input?: 'required' | 'optional';
  prompt?: string;
  guidance?: CancellationGuidance;
  pinned?: boolean;
};

export const CANCELLATION_TEXT_PROMPT = '남기고 싶은 의견';
export const CANCELLATION_TEXT_PLACEHOLDER = '자세히 남겨 주시면 개선에 큰 도움이 돼요';

export const CANCELLATION_REASONS: CancellationReasonOption[] = [
  {
    value: 'billing_change',
    label: '결제일이나 결제 수단을 바꾸려고 해요',
    input: 'optional',
    prompt: '무엇을 바꾸려고 하시나요?',
    guidance: 'payment_method',
  },
  {
    value: 'not_now',
    label: '당분간 글을 쓸 일이 없어요',
    input: 'optional',
    prompt: '나중에 다시 쓰신다면 무엇이 있으면 좋을까요?',
    guidance: 'waiver',
  },
  {
    value: 'low_usage',
    label: '생각보다 자주 쓰지 않아요',
    input: 'optional',
    prompt: '자주 쓰지 않게 된 이유가 있을까요?',
    guidance: 'waiver',
  },
  { value: 'expensive', label: '가격이 부담스러워요', input: 'optional', prompt: '어떤 조건이면 계속 쓰실 것 같나요?' },
  {
    value: 'unstable',
    label: '안정성이나 속도가 아쉬웠어요',
    input: 'optional',
    prompt: '어떤 상황에서 문제가 있었나요?',
    guidance: 'support',
  },
  {
    value: 'lack_features',
    label: '필요한 기능이 부족해요',
    input: 'optional',
    prompt: '어떤 기능이 필요하셨나요?',
    guidance: 'feature_request',
  },
  {
    value: 'switched',
    label: '다른 서비스를 쓰게 됐어요',
    input: 'optional',
    prompt: '어떤 서비스로 옮기셨나요? 더 좋았던 점도 알려 주세요',
  },
  { value: 'other', label: '직접 입력', input: 'required', prompt: '이유를 알려 주세요', pinned: true },
  { value: 'prefer_not_to_say', label: '답하고 싶지 않아요', pinned: true },
];

export type CancellationSurveyValue = {
  reason: CancellationReason;
  reason_other: string;
  detail: string;
};

export type CancellationSurveyDraft = {
  reason: CancellationReason | null;
  text: string;
};

export function orderCancellationReasons(random: () => number = Math.random): CancellationReasonOption[] {
  const shuffled = CANCELLATION_REASONS.filter((option) => !option.pinned).toSorted(() => random() - 0.5);
  const pinned = CANCELLATION_REASONS.filter((option) => option.pinned);

  return [...shuffled, ...pinned];
}

export function cancellationTextInput(reason: CancellationReason | null): 'required' | 'optional' | null {
  if (!reason) {
    return 'optional';
  }

  return CANCELLATION_REASONS.find((option) => option.value === reason)?.input ?? null;
}

export function canSubmitCancellationSurvey(draft: CancellationSurveyDraft): boolean {
  if (!draft.reason) {
    return false;
  }

  if (cancellationTextInput(draft.reason) === 'required') {
    return draft.text.trim() !== '';
  }

  return true;
}

export function buildCancellationSurveyValue(draft: CancellationSurveyDraft & { reason: CancellationReason }): CancellationSurveyValue {
  const text = cancellationTextInput(draft.reason) ? draft.text.trim() : '';

  if (draft.reason === 'other') {
    return { reason: 'other', reason_other: text, detail: '' };
  }

  return { reason: draft.reason, reason_other: '', detail: text };
}
