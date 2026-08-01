import { checkResearch, leakLineLint } from '../checks.ts';
import { RESEARCH_SCHEMA } from '../contracts.ts';
import { renderSearchHits, searchBackground } from '../search.ts';
import { runAgentStage } from './agent-stage.ts';
import type { SearchExecutor } from '../../../core/worker/agent-loop.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { Deliverable } from '../../../core/worker/deliverable.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { Workspace } from '../../../core/worker/workspace.ts';
import type { StageLedger } from '../ledger.ts';
import type { Research, ResolvedResearch } from '../types.ts';

export const createSearch = (ctx: RunContext): SearchExecutor | null => {
  const apiKey = ctx.env.EXA_API_KEY;
  if (!apiKey) return null;
  return async (query) => {
    const hits = await searchBackground({ apiKey, query });
    return { content: hits.length > 0 ? renderSearchHits(hits) : '검색 결과 없음', hits: hits.length };
  };
};

const RESEARCH_DELIVERABLE: Deliverable = {
  label: '조사 결과',
  submitName: 'submit_research',
  submitDescription:
    '조사 산출물을 확정 제출한다. 검사를 통과해야 접수된다. 모든 인용은 원고에서 글자 그대로여야 하고, 열람한 범위 안이어야 한다.',
  outputs: {
    'output/research.yaml': {
      schema: RESEARCH_SCHEMA,
      lints: [leakLineLint],
      description: '조사 결과 — 글의 성격·문체 관습·분석 경계의 확정',
    },
  },
};

// 규약의 확정. 제출은 반려하지 않고 정제+기록한다 — 자유 탐색 단계다.
export const runResearch = async (
  ctx: RunContext,
  clients: LlmClients,
  workspace: Workspace,
  manuscriptFile: string,
  search: SearchExecutor | null,
): Promise<{ research: ResolvedResearch; ledger: StageLedger }> => {
  const stage = await runAgentStage<Research, ResolvedResearch>(ctx, {
    clients,
    stage: 'research',
    prompt: ctx.prompts.research,
    workspace,
    manuscriptAccess: true,
    system: '',
    initial: '원고를 조사해 output/research.yaml을 작성하고 submit_research로 제출하세요.',
    search,
    deliverable: RESEARCH_DELIVERABLE,
    manuscriptFile,
    baseTools: [],
    onSubmit: (_path, value, { turn, tools, ledger, file }) => {
      const check = checkResearch(ctx.document.content, value, tools, file);
      for (const note of check.notes) ledger.events.push({ turn, kind: 'research-check', detail: note });
      return { accept: check.research, message: `접수. 코드 검증 ${check.notes.length}건 정제.` };
    },
  });

  return { research: stage.value, ledger: stage.ledger };
};
