import type { WorkflowStep } from 'cloudflare:workers';
import type { ItemDraft, PhasePrompt, Usage } from '../contracts.ts';

export type RunEnv = {
  CLOUDFLARE_API_KEY: string;
  CLOUDFLARE_AIGATEWAY_URL: string;
  CLOUDFLARE_AIGATEWAY_COMPAT_URL: string;
  EXA_API_KEY?: string;
};

// 세대는 DB에 직접 쓰지 않는다. 이 셋만 쓰고 산출물은 반환값으로 낸다.
export type RunContext = {
  step: WorkflowStep;
  env: RunEnv;
  // refId는 반입 원본 문서 식별자 — 워크스페이스의 원고 파일명(manuscript/<refId>.txt)이 된다.
  document: { id: string; refId: string; content: string };
  prompts: Record<string, PhasePrompt>;
  phase: (key: string) => Promise<void>;
  cached: <T>(key: string, fn: (usage: Usage) => Promise<T>) => Promise<{ value: T; cached: boolean }>;
  ledger: (key: string, value: unknown) => Promise<void>;
};

export type GenerationRunner = (ctx: RunContext) => Promise<{ items: ItemDraft[] }>;
