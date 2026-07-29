import type { D1Database, Workflow } from '@cloudflare/workers-types';

export { RunWorkflow } from './run-workflow.ts';
export { SamplingWorkflow } from './sampling.ts';

// 세대 무관 실행 — 원고 1편 × 프롬프트 묶음 1개. 나머지는 워크플로가 DB에서 읽는다.
// payload에 복제하면 DB와 어긋날 수 있다.
export type RunParams = { runId: string };
export type SamplingParams = { runId: string; size: number };

export type FlowEnv = {
  DB: D1Database;
  RUN: Workflow<RunParams>;
  SAMPLING: Workflow<SamplingParams>;
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

// 앱은 cross-script Workflow 바인딩으로만 이 워커를 구동한다 — fetch는 헬스체크뿐이다.
// eslint-disable-next-line import/no-default-export
export default {
  fetch: () => new Response('flows', { status: 200 }),
};
