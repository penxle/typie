import type { D1Database, Workflow } from '@cloudflare/workers-types';

export { AnalysisWorkflow } from './analysis.ts';
export { EditorialWorkflow } from './editorial.ts';
export { JudgeWorkflow } from './judge.ts';
export { PipelineWorkflow } from './pipeline.ts';
export { RulingWorkflow } from './ruling.ts';
export { SamplingWorkflow } from './sampling.ts';

export type SamplingParams = { runId: string; corpusVersion: string; size: number };
// 이미 저장된 실행(sourceRunId)의 피드백에 판정 단계만 다시 돌린다.
export type JudgeParams = { runId: string; promptSetId: string; sourceRunId: string; documentId: string };
// 판정 실행(sourceJudgeRunId)이 남긴 항변문에 다른 벤더의 심판만 다시 돌린다.
export type RulingParams = { runId: string; promptSetId: string; sourceJudgeRunId: string; documentId: string };
// Editorial 파이프라인 — 기존 분석과 동형 파라미터.
export type EditorialParams = { runId: string; promptSetId: string; variantLabel: string; corpusVersion: string; documentId: string };
export type AnalysisParams = {
  runId: string;
  promptSetId: string;
  variantLabel: string;
  corpusVersion: string;
  documentId: string;
};
export type PipelineParams = {
  runId: string;
  promptVariantId: string;
  variantLabel: string;
  corpusVersion: string;
  documentId: string;
};

export type FlowEnv = {
  DB: D1Database;
  SAMPLING: Workflow<SamplingParams>;
  PIPELINE: Workflow<PipelineParams>;
  ANALYSIS: Workflow<AnalysisParams>;
  JUDGE: Workflow<JudgeParams>;
  RULING: Workflow<RulingParams>;
  EDITORIAL: Workflow<EditorialParams>;
  INTERNAL_API_KEY: string;
  INTERNAL_API_BASE: string;
  CLOUDFLARE_API_KEY: string;
  CLOUDFLARE_AIGATEWAY_URL: string;
  // OpenAI 등 비Anthropic 모델용 통합 경로. Anthropic 전용 경로와 달리 {provider}/{model}
  // 표기를 그대로 받는다.
  CLOUDFLARE_AIGATEWAY_COMPAT_URL: string;
  // 2차 창작 원작 조사용. 없으면 배경 단계를 건너뛴다.
  EXA_API_KEY?: string;
};

// 앱(apps/eval)은 cross-script Workflow 바인딩(SAMPLING/PIPELINE)으로만 이 워커를 구동한다 — fetch는
// 헬스체크 이상의 역할이 없다.
// eslint-disable-next-line import/no-default-export
export default {
  fetch: () => new Response('flows', { status: 200 }),
};
