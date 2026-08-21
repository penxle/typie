// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import type { AskQuestion } from '@typie/prism';

export const subjectTitle = (subject: string | null): string => `「${subject || '새 대화'}」`;

export const askBody = (questions: AskQuestion[]): string => {
  const first = questions[0]?.question ?? '';
  return questions.length > 1 ? `${first} 외 ${questions.length - 1}개` : first;
};

export const pushKey = {
  ask: (toolCallId: string) => `prism:push:ask:${toolCallId}`,
};
