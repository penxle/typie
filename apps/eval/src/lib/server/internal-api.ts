import type { StageKey, StagePrompt } from '../domain/admin-types.ts';

// cspell:disable
const STAGE_PROMPT_IDS: Record<StageKey, string> = {
  summarize: 'PRMT0SUMMARIZE',
  meta: 'PRMT0META',
  analyze: 'PRMT0ANALYZE',
};
// cspell:enable

type ApiPrompt = { id: string; model: string; effort: string | null; systemPrompt: string; toolDescriptions: Record<string, unknown> };

const toStagePrompt = (prompt: ApiPrompt): StagePrompt => ({
  system: prompt.systemPrompt,
  tools: prompt.toolDescriptions,
  model: prompt.model,
  effort: prompt.effort,
});

export type CurrentPrompts = Record<StageKey, StagePrompt>;

export type InternalApi = {
  current: () => Promise<CurrentPrompts>;
  apply: (stage: StageKey, prompt: StagePrompt) => Promise<void>;
  stagePromptId: (stage: StageKey) => string;
};

export const createInternalApi = (base: string, key: string): InternalApi => {
  const headers = { authorization: `Bearer ${key}` };

  return {
    current: async () => {
      const response = await fetch(`${base}/internal/prompts`, { headers });
      if (!response.ok) {
        throw new Error(`prompts fetch failed: ${response.status}`);
      }

      const { prompts } = (await response.json()) as { prompts: ApiPrompt[] };
      const byId = new Map(prompts.map((p) => [p.id, p]));

      const result = {} as CurrentPrompts;
      for (const stage of Object.keys(STAGE_PROMPT_IDS) as StageKey[]) {
        const prompt = byId.get(STAGE_PROMPT_IDS[stage]);
        if (!prompt) {
          throw new Error(`prompt missing for stage: ${stage}`);
        }
        result[stage] = toStagePrompt(prompt);
      }
      return result;
    },

    apply: async (stage, prompt) => {
      const response = await fetch(`${base}/internal/prompts/${STAGE_PROMPT_IDS[stage]}`, {
        method: 'PUT',
        headers: { ...headers, 'content-type': 'application/json' },
        body: JSON.stringify({ model: prompt.model, effort: prompt.effort, systemPrompt: prompt.system, toolDescriptions: prompt.tools }),
      });
      if (!response.ok) {
        throw new Error(`prompt apply failed: ${response.status}`);
      }
    },

    stagePromptId: (stage) => STAGE_PROMPT_IDS[stage],
  };
};
