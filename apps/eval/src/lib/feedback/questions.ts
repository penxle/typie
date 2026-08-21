import type { AskAnswer, AskQuestion } from './live.ts';

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

// 카드가 읽을 답변 색인 — 답변 문면은 해소 이벤트(tool.resolved)가 실어 리듀서 엔트리에 남는다(live.ts). 답을 못
// 받고 끝난 질문은 색인에 넣지 않아, 답변 없음이 한 가지 부재로만 카드에 닿는다. 실행 중·종결 화면이 같은 규칙을 쓴다.
export const askAnswerIndex = (entries: { toolCallId: string; answers: AskAnswer[] | null }[] | null): Record<string, AskAnswer[]> =>
  Object.fromEntries((entries ?? []).flatMap((entry) => (entry.answers === null ? [] : [[entry.toolCallId, entry.answers] as const])));

export const buildAnswers = (questions: AskQuestion[], drafts: DraftAnswer[]): AskAnswer[] =>
  questions.map((question, i) => {
    const draft = drafts[i];
    const other = draft.otherOn && draft.other.trim().length > 0 ? [draft.other.trim()] : [];
    return { question: question.question, choice: [...draft.choices, ...other] };
  });
