// 재설계 파이프라인 판정 — 5점 척도 대신 피드백 하나하나에 예/아니오를 묻는다.
// 세 값 모두 null을 허용한다: null은 '아직 판정하지 않음'이다. 기본값을 통과로 두면
// 평가자가 손대지 않은 피드백과 통과시킨 피드백이 집계에서 구별되지 않는다.

export type VerdictAxis = { key: 'correct' | 'needed' | 'useful'; question: string; negative: string };

// 질문은 무엇을 재는지가 문장만으로 읽혀야 한다. 세 축은 각각 사실·가치·행동 가능성이다.
export const FEEDBACK_VERDICT_AXES: VerdictAxis[] = [
  { key: 'correct', question: '본문을 정확히 읽었나요?', negative: '본문에 없는 것을 말했거나 잘못 읽었습니다' },
  { key: 'needed', question: '짚을 만한 내용인가요?', negative: '맞는 말이지만 굳이 말할 일은 아닙니다' },
  { key: 'useful', question: '작가가 무엇을 할지 알 수 있나요?', negative: '읽어도 어떻게 손대야 할지 모르겠습니다' },
];

export type ReviewVerdictAxis = { key: 'readCorrectly' | 'priorityUseful'; question: string; negative: string };

export const REVIEW_VERDICT_AXES: ReviewVerdictAxis[] = [
  { key: 'readCorrectly', question: '이 작품을 제대로 파악했나요?', negative: '작품이 무엇을 하려는 글인지 잘못 봤습니다' },
  { key: 'priorityUseful', question: '어디서부터 손댈지 납득되나요?', negative: '제시한 순서에 동의할 수 없습니다' },
];

export type Verdict = boolean | null;

export type FeedbackVerdict = { correct: Verdict; needed: Verdict; useful: Verdict; note?: string };
export type FeedbackVerdictMap = Record<string, FeedbackVerdict>;

export type ReviewVerdict = { readCorrectly: Verdict; priorityUseful: Verdict; note?: string };
export type ReviewVerdictMap = Record<string, ReviewVerdict>;

export const EMPTY_FEEDBACK_VERDICT: FeedbackVerdict = { correct: null, needed: null, useful: null };
export const EMPTY_REVIEW_VERDICT: ReviewVerdict = { readCorrectly: null, priorityUseful: null };

// 아무 축도 답하지 않고 메모도 없으면 저장할 것이 없다.
export const isEmptyFeedbackVerdict = (v: FeedbackVerdict): boolean =>
  v.correct === null && v.needed === null && v.useful === null && !v.note;
export const isEmptyReviewVerdict = (v: ReviewVerdict): boolean => v.readCorrectly === null && v.priorityUseful === null && !v.note;

// 배정받은 글은 피드백 전부에 세 축을 다 답해야 제출된다. 일부만 답한 판정은 비율을 계산할 수
// 없게 만든다 — 문제 있는 것만 표시하면 '예'가 과소 집계되어 분모가 무너지기 때문이다.
// 시간이 부족하면 편 수를 줄이는 것이 맞지, 한 편 안에서 건너뛰는 것은 데이터를 버리는 일이다.
export const isFeedbackComplete = (v: FeedbackVerdict | undefined): boolean =>
  v !== undefined && v.correct !== null && v.needed !== null && v.useful !== null;

export const isReviewComplete = (v: ReviewVerdict | undefined): boolean =>
  v !== undefined && v.readCorrectly !== null && v.priorityUseful !== null;

// 제출 가능 여부. 화면과 서버가 같은 판단을 쓰도록 여기 한 곳에 둔다.
export const incompleteFeedbackIds = (feedbackIds: string[], verdicts: FeedbackVerdictMap): string[] =>
  feedbackIds.filter((id) => !isFeedbackComplete(verdicts[id]));

export const hasRejection = (v: FeedbackVerdict | undefined): boolean =>
  v !== undefined && (v.correct === false || v.needed === false || v.useful === false);

// 작품 총평. 파이프라인이 넣은 그대로라 스키마 밖 값이 섞일 수 있어 방어적으로 읽는다.
// feedbackIndexes는 세트 안 피드백 순번이며 0-based다(실행 데이터로 확인). 화면 번호는 1-based라
// 표시할 때 1을 더한다. 값이 아예 없는 실행도 있어 빈 배열을 허용한다.
export type WorkReview = {
  characterization: string;
  strengths: string;
  patterns: { theme: string; body: string; feedbackIndexes: number[] }[];
  priority: { body: string; feedbackIndexes: number[] }[];
};

const asText = (value: unknown): string => (typeof value === 'string' ? value : '');

const asIndexes = (value: unknown): number[] =>
  Array.isArray(value) ? [...new Set(value.filter((n): n is number => Number.isSafeInteger(n) && n >= 0))] : [];

export const parseWorkReview = (raw: unknown): WorkReview | null => {
  if (!raw || typeof raw !== 'object') return null;
  const source = raw as Record<string, unknown>;
  return {
    characterization: asText(source.characterization),
    strengths: asText(source.strengths),
    patterns: (Array.isArray(source.patterns) ? source.patterns : [])
      .map((p) => ({
        theme: asText((p as Record<string, unknown>)?.theme),
        body: asText((p as Record<string, unknown>)?.body),
        feedbackIndexes: asIndexes((p as Record<string, unknown>)?.feedbackIndexes),
      }))
      .filter((p) => p.body),
    priority: (Array.isArray(source.priority) ? source.priority : [])
      .map((p) => ({
        body: asText((p as Record<string, unknown>)?.body),
        feedbackIndexes: asIndexes((p as Record<string, unknown>)?.feedbackIndexes),
      }))
      .filter((p) => p.body),
  };
};
