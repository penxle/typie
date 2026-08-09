import type { AskAnswer, AskQuestion } from './live.ts';
import type { ReviewQuestionRecord } from './types.ts';

export type DraftAnswer = { choices: string[]; other: string; otherOn: boolean };

export const emptyDrafts = (questions: AskQuestion[]): DraftAnswer[] => questions.map(() => ({ choices: [], other: '', otherOn: false }));

export const toggleChoice = (question: AskQuestion, draft: DraftAnswer, label: string): DraftAnswer =>
  question.multi
    ? { ...draft, choices: draft.choices.includes(label) ? draft.choices.filter((c) => c !== label) : [...draft.choices, label] }
    : { choices: [label], other: draft.other, otherOn: false };

export const toggleOther = (question: AskQuestion, draft: DraftAnswer): DraftAnswer =>
  question.multi ? { ...draft, otherOn: !draft.otherOn } : { choices: [], other: draft.other, otherOn: true };

export const isAnswered = (draft: DraftAnswer): boolean => draft.choices.length > 0 || (draft.otherOn && draft.other.trim().length > 0);

export const answeredAll = (drafts: DraftAnswer[]): boolean => drafts.every(isAnswered);

// 종결 리뷰에서 카드가 읽을 답변 색인 — 답변 문면은 재생에 없고 사영 기록에만 남으므로(types.ts) 거기서 되짚는다.
// 답을 못 받고 끝난 질문(closed)은 색인에 넣지 않아, 답변 없음이 한 가지 부재로만 카드에 닿는다. 종결 화면 두 곳
// (세션 타임라인·과정)이 같은 규칙을 쓰도록 파생을 여기 한 곳에 둔다.
export const askAnswerIndex = (records: ReviewQuestionRecord[] | null): Record<string, AskAnswer[]> =>
  Object.fromEntries((records ?? []).flatMap((record) => (record.answers === null ? [] : [[record.toolCallId, record.answers] as const])));

export const buildAnswers = (questions: AskQuestion[], drafts: DraftAnswer[]): AskAnswer[] =>
  questions.map((question, i) => {
    const draft = drafts[i];
    const other = draft.otherOn && draft.other.trim().length > 0 ? [draft.other.trim()] : [];
    return { question: question.question, choice: [...draft.choices, ...other] };
  });
