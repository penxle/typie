// prism 계약 미러 — 원본: prism apps/feedback/result.ts의 FeedbackResult, core/terminal.ts의 foldUsage,
// core/do.ts의 WorkflowView. prism 쪽 개정 시 이 파일을 함께 갱신한다.

import type { AskAnswer, AskQuestion } from './live.ts';

export type Anchor = { start: number; end: number; head: string; tail: string };

export type FeedbackIssue = { axis: string; pass: 'critique' | 'proofread'; body: string | null; anchors: Anchor[] };

export type FeedbackConclusion = {
  understanding: string | null;
  strengths: (Anchor & { body: string | null })[];
  clearances: { axis: string; note: string }[];
  patterns: { theme: string | null; body: string; issues: number[] }[];
  priorities: { body: string; issues: number[] }[];
};

export type FeedbackResult = {
  version: 1;
  issues: FeedbackIssue[];
  // low 티어 결과에는 키 자체가 없다 — 마무리 글을 쓰는 에이전트가 그 티어에 없다.
  conclusion?: FeedbackConclusion;
};

export type UsageFold = {
  provider: string;
  agent: string;
  model: string;
  effort: string | null;
  turns: number;
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  thinkingTokens: number | null;
};

export type RunUsage = { complete: boolean; folds: UsageFold[] };

// prism 와이어 — settled가 판별자다. DB(Reviews.usage)에는 판별자를 벗긴 RunUsage만 남는다.
export type WorkflowUsage = ({ settled: true } & RunUsage) | { settled: false; folds: UsageFold[] };

export type PrismWorkflow = {
  status: 'running' | 'completed' | 'failed' | 'canceled';
  result: FeedbackResult | null;
  error: string | null;
  usage: WorkflowUsage | null;
  startedAt: number;
  finishedAt: number | null;
};

export type PrismWorkflowView = { workflow: PrismWorkflow };

// 사영이 굳히는 질문 기록 — 카드 자체는 이벤트 재생이 세우므로, 굳이 따로 남기는 이유는 재생으로 복원할 수 없는
// 답변 문면 하나다(이벤트에 없고 prism 원장에만 있다). 나머지 필드는 그 문면의 귀속처이자 기록의 자족성이다.
export type ReviewQuestionRecord = {
  agentName: string;
  toolCallId: string;
  stage: string | null;
  at: number | null;
  status: 'answered' | 'closed';
  questions: AskQuestion[];
  answers: AskAnswer[] | null;
};
