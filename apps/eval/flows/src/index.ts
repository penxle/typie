import type { D1Database, Workflow } from '@cloudflare/workers-types';

export { AnalysisWorkflow } from './analysis.ts';
export { PipelineWorkflow } from './pipeline.ts';
export { SamplingWorkflow } from './sampling.ts';

export type SamplingParams = { runId: string; corpusVersion: string; size: number };
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
  INTERNAL_API_KEY: string;
  INTERNAL_API_BASE: string;
  CLOUDFLARE_API_KEY: string;
  CLOUDFLARE_AIGATEWAY_URL: string;
  // 2차 창작 원작 조사용. 없으면 배경 단계를 건너뛴다.
  EXA_API_KEY?: string;
};

// 앱(apps/eval)은 cross-script Workflow 바인딩(SAMPLING/PIPELINE)으로만 이 워커를 구동한다 — fetch는
// 헬스체크 이상의 역할이 없다.
// eslint-disable-next-line import/no-default-export
export default {
  fetch: () => new Response('flows', { status: 200 }),
};
