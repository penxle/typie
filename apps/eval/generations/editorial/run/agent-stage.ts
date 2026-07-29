import { schemaViolations } from '../../../core/tool-schema.ts';
import { executeToolUses, runTurn } from '../../../core/worker/agent-loop.ts';
import { LLM_STEP } from '../../../core/worker/llm.ts';
import { hasToolSyntaxLeak, LEAK_STREAK_MAX, TURN_CAP } from '../checks.ts';
import { emptyLedger } from '../ledger.ts';
import { renderRejection } from '../render.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { PhasePrompt, ToolRecord } from '../../../core/contracts.ts';
import type { SearchExecutor, ToolUse, TurnOutput } from '../../../core/worker/agent-loop.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { StageLedger } from '../ledger.ts';

// 직렬화 사고는 strict 스키마도 뚫는다(실측: protected 필드 누락 제출로 검증 크래시).
// 오형 제출은 처리 전에 걸러 반려 루프로 보낸다 — 검증 코드는 정형 입력만 전제한다.
export const shapeRejection = (tool: { input_schema: unknown }, input: unknown): string[] | null => {
  const violations = schemaViolations(tool.input_schema, input);
  return violations.length > 0
    ? ['제출이 스키마와 다릅니다 — 필드 형태를 스키마 그대로 지켜 다시 제출하세요', ...violations.slice(0, 5)]
    : null;
};

// 제출 처리 결과. done이 설정되면 이 턴의 나머지 결과를 반영한 뒤 단계를 끝낸다.
export type SubmissionOutcome<T> = { done?: T; results: { toolUseId: string; content: string }[] };

export type AgentStageOptions<T> = {
  client: Anthropic;
  stage: string;
  prompt: PhasePrompt;
  tools: Anthropic.Messages.Tool[];
  system: string;
  initial: string;
  search: SearchExecutor | null;
  // 이전 단계에서 넘어온 도구 기록 — 열람 범위 검증이 단계를 넘어 이어진다(계획 초안에서 읽은
  // 대목을 수정 라운드에서 인용하는 경우).
  baseTools: ToolRecord[];
  // 원장을 남길 키와 누산기. 한 매니페스트 단계가 내부적으로 여러 라운드로 갈릴 때
  // 라운드마다 따로 남기면 같은 도구가 중복되고 화면의 단계 순서도 무너진다.
  ledgerKey?: string;
  ledger?: StageLedger;
  onSubmissions: (subs: ToolUse[], turn: number, tools: ToolRecord[], ledger: StageLedger) => SubmissionOutcome<T>;
};

// 에이전틱 단계의 공통 루프. 턴 하나 = 스텝 하나, 턴당 D1 캐시. 원장은 캐시된 도구 실행
// 결과에서 매 리플레이 재구성된다 — 순수해야 하는 이유다.
export const runAgentStage = async <T>(ctx: RunContext, options: AgentStageOptions<T>): Promise<{ value: T; ledger: StageLedger }> => {
  const { client, stage, prompt, tools, system, initial, search, baseTools, onSubmissions } = options;
  const content = ctx.document.content;
  const ledger = options.ledger ?? emptyLedger();
  const ledgerKey = options.ledgerKey ?? stage;
  const messages: Anthropic.MessageParam[] = [{ role: 'user', content: initial }];
  let leakStreak = 0;

  for (let turn = 0; turn < TURN_CAP; turn++) {
    // step.do의 반환 타입은 Serializable로 좁혀져 있어 unknown을 품은 구조를 그대로 통과시키지
    // 못한다. 값은 실제로 JSON 왕복이 되므로 경계에서만 단언한다.
    const turnStep = (await ctx.step.do(`${stage}-${turn}`, LLM_STEP, async () => {
      const { value, cached } = await ctx.cached<TurnOutput>(`${stage}/turn/${turn}`, (usage) =>
        runTurn(client, prompt, tools, system, messages, usage),
      );
      return { out: value, cached } as never;
    })) as unknown as { out: TurnOutput; cached: boolean };

    const out = turnStep.out;
    messages.push({ role: 'assistant', content: out.content as Anthropic.MessageParam['content'] });

    if (out.toolUses.length === 0) {
      messages.push({ role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' });
      continue;
    }

    // 검색이 비결정적이므로 도구 실행 전체를 캐시한다 — 리플레이가 같은 결과를 재사용한다.
    const executed = (await ctx.step.do(`${stage}-tools-${turn}`, async () => {
      const { value } = await ctx.cached(`${stage}/tools/${turn}`, () => executeToolUses(content, out.toolUses, turn, search));
      return value as never;
    })) as unknown as Awaited<ReturnType<typeof executeToolUses>>;

    ledger.tools.push(...executed.records);

    const combinedTools = [...baseTools, ...ledger.tools];

    // 직렬화 오염 제출은 핸들러 앞에서 중앙 차단한다. 오염 문면이 대화에 남으면 이후 제출이
    // 그 형태를 모방하므로 원문을 컨텍스트에서 제거하고 처음부터 다시 쓰게 한다. 연속 오염은
    // 스테이지를 중단해 턴 낭비를 끊는다 — 회수는 캐시 리플레이 재실행.
    const cleanSubs: ToolUse[] = [];
    const leakResults: { toolUseId: string; content: string }[] = [];
    for (const sub of executed.submissions) {
      if (!hasToolSyntaxLeak([JSON.stringify(sub.input)])) {
        leakStreak = 0;
        cleanSubs.push(sub);
        continue;
      }
      leakStreak += 1;
      const input = sub.input as Record<string, unknown>;
      const excerpt = String(typeof input?.quoteStart === 'string' ? input.quoteStart : (input?.intent ?? '')).slice(0, 30);
      ledger.events.push({ turn, kind: 'leak-rejected', detail: `${sub.name}: ${excerpt}` });
      ledger.leaked.push({ turn, name: sub.name, input: sub.input });
      // 라이브 턴에서만 중단한다. 리플레이(캐시 턴)에서 발동하면 자력 회복하고 완주했던 실행의
      // 회수가 영구히 막힌다 — 실측: 7연속 오염 후 회복·완주 사례 존재.
      if (leakStreak > LEAK_STREAK_MAX && !turnStep.cached) {
        throw new Error(`${stage}: 직렬화 오염 연속 ${leakStreak}회 — 컨텍스트 오염으로 스테이지 중단`);
      }
      leakResults.push({
        toolUseId: sub.id,
        content: renderRejection([
          '제출 필드에 도구 호출 구문이 혼입되어 원문을 대화에서 제거했습니다',
          `제출 식별: ${excerpt || sub.name}`,
          '직전 제출 문면을 참조하지 말고, 같은 내용을 처음부터 순수 텍스트로 새로 작성해 제출하세요',
        ]),
      });
      const last = messages.at(-1);
      if (last?.role === 'assistant' && Array.isArray(last.content)) {
        for (const block of last.content) {
          if (block.type === 'tool_use' && block.id === sub.id) {
            block.input = { scrubbed: '직렬화 오염 제출 — 원문 제거됨' };
          }
        }
      }
    }

    const outcome = onSubmissions(cleanSubs, turn, combinedTools, ledger);

    const resultOf = new Map<string, string>();
    for (const r of executed.results) resultOf.set(r.toolUseId, r.content);
    for (const r of leakResults) resultOf.set(r.toolUseId, r.content);
    for (const r of outcome.results) resultOf.set(r.toolUseId, r.content);
    messages.push({
      role: 'user',
      content: out.toolUses.map((use) => ({
        type: 'tool_result' as const,
        tool_use_id: use.id,
        content: resultOf.get(use.id) ?? '처리되지 않은 호출',
      })),
    });

    // 단계가 끝나야 원장이 남으면 도는 동안 무엇을 읽고 무엇이 걸렸는지 볼 수 없다.
    // 턴마다 덮어써 진행 중에도 최신 상태가 보이게 한다.
    await ctx.ledger(`ledger/${ledgerKey}`, ledger);

    if (outcome.done !== undefined) return { value: outcome.done, ledger };
  }

  throw new Error(`${stage}: 턴 백스톱(${TURN_CAP}) 초과`);
};
