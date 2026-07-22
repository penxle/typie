export type FeedbackLabelKind = 'negative' | 'positive';
export type FeedbackLabel = { key: string; name: string; kind: FeedbackLabelKind };

export const FEEDBACK_LABELS: FeedbackLabel[] = [
  { key: 'fact-error', name: '사실 오인', kind: 'negative' },
  { key: 'scene-break-fp', name: '장면전환 오탐', kind: 'negative' },
  { key: 'generic', name: '일반론·뻔한 지적', kind: 'negative' },
  { key: 'style-push', name: '스타일 강요', kind: 'negative' },
  { key: 'repetition-oversensitive', name: '반복 과민', kind: 'negative' },
  { key: 'expression-issue', name: '표현 문제', kind: 'negative' },
  { key: 'etc', name: '기타', kind: 'negative' },
  { key: 'key-insight', name: '핵심 지적', kind: 'positive' },
  { key: 'novel-insight', name: '새로운 통찰', kind: 'positive' },
];

export const FEEDBACK_LABEL_KEYS = new Set(FEEDBACK_LABELS.map((l) => l.key));
export const NEGATIVE_LABEL_KEYS = new Set(FEEDBACK_LABELS.filter((l) => l.kind === 'negative').map((l) => l.key));

export type FeedbackLabelEntry = { labels: string[]; comment?: string };
export type FeedbackLabelMap = Record<string, FeedbackLabelEntry>;

export const deriveFalsePositiveIds = (map: FeedbackLabelMap): string[] =>
  Object.entries(map)
    .filter(([, entry]) => entry.labels.some((key) => NEGATIVE_LABEL_KEYS.has(key)))
    .map(([feedbackId]) => feedbackId);
