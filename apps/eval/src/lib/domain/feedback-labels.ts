export type FeedbackLabelKind = 'negative' | 'positive' | 'system';
export type FeedbackLabel = { key: string; name: string; kind: FeedbackLabelKind; group: string };

// 층위 체계: 진단이 참인가(오독) → 판단이 온당한가(판단 오류) → 쓸모·전달(가치·형식). 시스템은 프롬프트 지표에서 제외되는 별도 축.
export const FEEDBACK_LABELS: FeedbackLabel[] = [
  { key: 'misread', name: '본문 오독', kind: 'negative', group: '오독' },
  { key: 'context-missed', name: '맥락 놓침', kind: 'negative', group: '오독' },
  { key: 'intent-ignored', name: '의도 무시', kind: 'negative', group: '판단 오류' },
  { key: 'convention-ignored', name: '관례 무시', kind: 'negative', group: '판단 오류' },
  { key: 'style-push', name: '스타일 강요', kind: 'negative', group: '판단 오류' },
  { key: 'oversensitive', name: '과민 지적', kind: 'negative', group: '판단 오류' },
  { key: 'generic', name: '일반론', kind: 'negative', group: '가치·형식' },
  { key: 'no-action', name: '감상뿐', kind: 'negative', group: '가치·형식' },
  { key: 'delivery-issue', name: '표현·언어 문제', kind: 'negative', group: '가치·형식' },
  { key: 'anchor-issue', name: '위치 오류', kind: 'system', group: '시스템·기타' },
  { key: 'etc', name: '기타', kind: 'negative', group: '시스템·기타' },
  { key: 'key-insight', name: '핵심 지적', kind: 'positive', group: '긍정' },
  { key: 'novel-insight', name: '새로운 통찰', kind: 'positive', group: '긍정' },
  { key: 'actionable', name: '즉시 적용 가능', kind: 'positive', group: '긍정' },
];

// 라운드 1 저장 데이터의 구 키 — 프리셋에는 노출하지 않고 집계·표시 호환만 유지한다.
export const LEGACY_FEEDBACK_LABELS: FeedbackLabel[] = [
  { key: 'fact-error', name: '사실 오인', kind: 'negative', group: '오독' },
  { key: 'scene-break-fp', name: '장면전환 오탐', kind: 'negative', group: '판단 오류' },
  { key: 'repetition-oversensitive', name: '반복 과민', kind: 'negative', group: '판단 오류' },
  { key: 'expression-issue', name: '표현 문제', kind: 'negative', group: '가치·형식' },
];

export const ALL_FEEDBACK_LABELS: FeedbackLabel[] = [...FEEDBACK_LABELS, ...LEGACY_FEEDBACK_LABELS];

export const FEEDBACK_LABEL_KEYS = new Set(FEEDBACK_LABELS.map((l) => l.key));
export const NEGATIVE_LABEL_KEYS = new Set(ALL_FEEDBACK_LABELS.filter((l) => l.kind === 'negative').map((l) => l.key));
export const SYSTEM_LABEL_KEYS = new Set(ALL_FEEDBACK_LABELS.filter((l) => l.kind === 'system').map((l) => l.key));

// "오탐"은 진단 자체가 거짓인 오독 계열로 한정한다. 구 키 2종은 라운드 1 수치 보존용.
export const STRICT_FALSE_POSITIVE_KEYS = new Set(['misread', 'context-missed', 'fact-error', 'scene-break-fp']);

// 판단 오류 = 진단은 참이나 평가가 부당한 계열. 구 키 2종은 라운드 1 소급 집계용.
export const JUDGMENT_ERROR_KEYS = new Set([
  'intent-ignored',
  'convention-ignored',
  'style-push',
  'oversensitive',
  'scene-break-fp',
  'repetition-oversensitive',
]);

export type FeedbackLabelEntry = { labels: string[]; comment?: string };
export type FeedbackLabelMap = Record<string, FeedbackLabelEntry>;

export const deriveFalsePositiveIds = (map: FeedbackLabelMap): string[] =>
  Object.entries(map)
    .filter(([, entry]) => entry.labels.some((key) => NEGATIVE_LABEL_KEYS.has(key)))
    .map(([feedbackId]) => feedbackId);
