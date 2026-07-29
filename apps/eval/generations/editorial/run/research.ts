import { checkResearch, hasToolSyntaxLeak } from '../checks.ts';
import { GREP_TOOL, READ_TOOL, SEARCH_TOOL, SUBMIT_RESEARCH_TOOL } from '../contracts.ts';
import { renderRejection } from '../render.ts';
import { renderSearchHits, searchBackground } from '../search.ts';
import { runAgentStage, shapeRejection } from './agent-stage.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { SearchExecutor } from '../../../core/worker/agent-loop.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { StageLedger } from '../ledger.ts';
import type { ResolvedResearch } from '../types.ts';

export const createSearch = (ctx: RunContext): SearchExecutor | null => {
  const apiKey = ctx.env.EXA_API_KEY;
  if (!apiKey) return null;
  return async (query) => {
    const hits = await searchBackground({ apiKey, query });
    return { content: hits.length > 0 ? renderSearchHits(hits) : '검색 결과 없음', hits: hits.length };
  };
};

// 규약의 확정. 제출은 반려하지 않고 정제+기록한다 — 자유 탐색 단계다.
export const runResearch = async (
  ctx: RunContext,
  client: Anthropic,
  search: SearchExecutor | null,
): Promise<{ research: ResolvedResearch; ledger: StageLedger }> => {
  const stage = await runAgentStage<ResolvedResearch>(ctx, {
    client,
    stage: 'research',
    prompt: ctx.prompts.research,
    tools: [READ_TOOL, GREP_TOOL, SEARCH_TOOL, SUBMIT_RESEARCH_TOOL],
    system: '',
    initial: '원고를 조사하고 submit_research로 제출하세요.',
    search,
    baseTools: [],
    onSubmissions: (subs, turn, tools, ledger) => {
      const results: { toolUseId: string; content: string }[] = [];
      let done: ResolvedResearch | undefined;
      for (const sub of subs) {
        const shape = shapeRejection(SUBMIT_RESEARCH_TOOL, sub.input);
        if (shape) {
          ledger.events.push({ turn, kind: 'research-check', detail: `오형 제출 반려: ${shape[1] ?? ''}`.slice(0, 120) });
          results.push({ toolUseId: sub.id, content: renderRejection(shape) });
          continue;
        }
        // 오염된 규약은 전 하류를 오염시킨다 — 조사 제출도 직렬화 사고를 반려한다.
        if (hasToolSyntaxLeak([JSON.stringify(sub.input)])) {
          ledger.events.push({ turn, kind: 'research-check', detail: '제출에 도구 구문 누출 — 반려' });
          results.push({
            toolUseId: sub.id,
            content: renderRejection(['필드에 도구 호출 구문이 섞임 — 각 필드를 순수 텍스트로 다시 제출하라']),
          });
          continue;
        }
        const check = checkResearch(ctx.document.content, sub.input as never, tools);
        for (const note of check.notes) ledger.events.push({ turn, kind: 'research-check', detail: note });
        done = check.research;
        results.push({ toolUseId: sub.id, content: `접수. 코드 검증 ${check.notes.length}건 정제.` });
      }
      return { done, results };
    },
  });

  return { research: stage.value, ledger: stage.ledger };
};
