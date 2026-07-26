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
  // 공개 조건을 통과한 문서만 돌려준다 — 여기서 빠진 id는 비공개이거나 열람 제한이 걸린 글이다.
  publicTexts: (documentIds: string[]) => Promise<{ documentId: string }[]>;
  // 코퍼스에 저장되는 본문은 이 프로즈다. document_contents의 평문과 달라서, 표집 경로와 같은
  // 것을 쓰지 않으면 오라클이 코퍼스와 다른 형태의 글을 읽게 된다.
  extract: (documentIds: string[]) => Promise<{ documentId: string; prose: string | null }[]>;
};

// 두 엔드포인트 모두 요청당 상한이 있다(texts 50, extract 5).
const TEXTS_BATCH = 50;
const EXTRACT_BATCH = 5;

const chunk = <T>(items: T[], size: number): T[][] => {
  const out: T[][] = [];
  for (let i = 0; i < items.length; i += size) {
    out.push(items.slice(i, i + size));
  }
  return out;
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

    publicTexts: async (documentIds) => {
      const results: { documentId: string }[] = [];
      for (const batch of chunk(documentIds, TEXTS_BATCH)) {
        const response = await fetch(`${base}/internal/corpus/texts`, {
          method: 'POST',
          headers: { ...headers, 'content-type': 'application/json' },
          body: JSON.stringify({ documentIds: batch }),
        });
        if (!response.ok) {
          throw new Error(`corpus texts failed: ${response.status}`);
        }
        const { texts } = (await response.json()) as { texts: { documentId: string }[] };
        results.push(...texts);
      }
      return results;
    },

    extract: async (documentIds) => {
      const results: { documentId: string; prose: string | null }[] = [];
      for (const batch of chunk(documentIds, EXTRACT_BATCH)) {
        const response = await fetch(`${base}/internal/corpus/extract`, {
          method: 'POST',
          headers: { ...headers, 'content-type': 'application/json' },
          body: JSON.stringify({ documentIds: batch }),
        });
        if (!response.ok) {
          throw new Error(`corpus extract failed: ${response.status}`);
        }
        const body = (await response.json()) as { results: { documentId: string; prose: string | null }[] };
        results.push(...body.results);
      }
      return results;
    },
  };
};
