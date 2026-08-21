import type { AskAnswer, AskQuestion } from '@typie/prism';

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

export const buildAnswers = (questions: AskQuestion[], drafts: DraftAnswer[]): AskAnswer[] =>
  questions.map((question, i) => {
    const draft = drafts[i];
    const other = draft.otherOn && draft.other.trim().length > 0 ? [draft.other.trim()] : [];
    return { question: question.question, choice: [...draft.choices, ...other] };
  });
