export type StageKey = 'summarize' | 'meta' | 'analyze';
export type StagePrompt = { system: string; tools: Record<string, unknown>; model: string; effort: string | null };
export type VariantContent = Record<StageKey, StagePrompt>;
export type VariantStatus = 'draft' | 'ran' | 'adopted' | 'applied';
export type RunKind = 'sampling' | 'pipeline';
export type RunStatus = 'running' | 'succeeded' | 'failed' | 'cancelled';
export type RunPhase = 'candidates' | 'classify' | 'extract' | 'freeze';
export type RunDocStatus = 'pending' | 'running' | 'done' | 'failed' | 'cancelled';
