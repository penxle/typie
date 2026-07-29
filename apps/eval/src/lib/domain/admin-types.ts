export type StageKey = 'summarize' | 'meta' | 'analyze';
export type StagePrompt = {
  system: string;
  tools: Record<string, unknown>;
  model: string;
  effort: string | null;
  // 생략하면 provider 기본값(대개 1.0)으로 돈다 — 값을 넣은 단계만 해시가 바뀌므로 기존 캐시는 유지된다.
  temperature?: number | null;
};
export type VariantContent = Record<StageKey, StagePrompt>;
export type VariantStatus = 'draft' | 'ran' | 'adopted' | 'applied';
export type RunKind = 'sampling' | 'pipeline' | 'analysis' | 'judge';
export type RunStatus = 'running' | 'succeeded' | 'failed' | 'cancelled';
export type RunPhase = 'candidates' | 'classify' | 'extract' | 'freeze';
export type RunDocStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled';
