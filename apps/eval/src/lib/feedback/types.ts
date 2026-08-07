// prism 계약 미러 — 원본: prism/src/types.ts:12(Usage), apps/feedback/result.ts:9(FeedbackResult),
// session/workflow.ts:55(Fold), session/do.ts:153(RunRow). prism 쪽 개정 시 이 파일을 함께 갱신한다.

export type Anchor = { start: number; end: number; head: string; tail: string };

export type FeedbackIssue = { axis: string; pass: 'critique' | 'proofread'; body: string | null; anchors: Anchor[] };

export type FeedbackResult = {
  version: 1;
  issues: FeedbackIssue[];
  conclusion: {
    understanding: string | null;
    strengths: (Anchor & { body: string | null })[];
    clearances: { axis: string; note: string }[];
    patterns: { theme: string | null; body: string; issues: number[] }[];
    priorities: { body: string; issues: number[] }[];
  };
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

export type PrismRun = {
  runSeq: number;
  status: 'running' | 'completed' | 'failed' | 'canceled';
  result: FeedbackResult | null;
  error: string | null;
  usage: RunUsage | null;
  startedAt: number | null;
  finishedAt: number | null;
};

export type PrismSessionView = { session: Record<string, unknown>; runs: PrismRun[] };
