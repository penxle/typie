// 순수 — env·DB·네트워크 import 없음(node:test 직접 로드)
import { effectiveResolver } from '@typie/prism';
import type { ToolPolicy } from '@typie/prism';

export const subjectTitle = (subject: string | null): string => `「${subject || '새 대화'}」`;

const askQuestions = (data: unknown): string[] => {
  if (typeof data !== 'object' || data === null) return [];
  const raw = (data as { questions?: unknown }).questions;
  if (!Array.isArray(raw)) return [];
  return raw.flatMap((item) => {
    const question = (item as { question?: unknown } | null)?.question;
    return typeof question === 'string' ? [question] : [];
  });
};

export const pushCopy = (tool: string, data: unknown, subject: string | null): { title: string; body: string } => {
  const subjectLabel = subjectTitle(subject);

  if (tool === 'ask-user') {
    const questions = askQuestions(data);
    const first = questions[0] ?? '';
    const body = questions.length > 1 ? `${first} 외 ${questions.length - 1}개` : first;
    return {
      title: `질문이 있어요 — ${subjectLabel}`,
      body: body === '' ? '열어서 확인해 주세요.' : body,
    };
  }

  if (tool === 'confirm-review') {
    return { title: `리뷰를 시작할까요? — ${subjectLabel}`, body: '원고와 살펴볼 깊이를 확인해 주세요.' };
  }

  return { title: `확인이 필요해요 — ${subjectLabel}`, body: '프리즘이 승인을 기다리고 있어요.' };
};

export const pushKey = {
  ask: (toolCallId: string) => `prism:push:ask:${toolCallId}`,
};

export const shouldPushAsk = (tool: string, policy: ToolPolicy): boolean => effectiveResolver(tool, policy) === 'user';
