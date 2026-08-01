// 에이전틱 스테이지 루프. 역할은 오케스트레이션뿐이다: 턴을 돌리고, 파일 도구를 워크스페이스에
// 순수 적용하고, 검색을 캐시로 실행하고, 제출을 게이트한 뒤 도메인 수용(onSubmit) 하나에
// 넘긴다. 산출물이 무엇인지는 deliverable이, 그것을 받을지는 onSubmit이 정한다.
//
// 턴 하나 = 스텝 하나, 턴당 D1 캐시. 원장은 캐시된 턴 출력에서 매 리플레이 재구성된다 —
// 여기서 하는 모든 일이 결정적이어야 하는 이유다.
import { executeSearches, runTurn } from '../../../core/worker/agent-loop.ts';
import { deliverableGuide, deliverableTools, finalizeHeader, validateOutput } from '../../../core/worker/deliverable.ts';
import { LLM_STEP } from '../../../core/worker/llm.ts';
import { TURN_CAP } from '../checks.ts';
import { SEARCH_TOOL } from '../contracts.ts';
import { emptyLedger, turnNote } from '../ledger.ts';
import { renderRejection } from '../render.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { PhasePrompt, ToolRecord } from '../../../core/contracts.ts';
import type { SearchExecutor, TurnOutput } from '../../../core/worker/agent-loop.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { Deliverable } from '../../../core/worker/deliverable.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { Workspace } from '../../../core/worker/workspace.ts';
import type { StageLedger } from '../ledger.ts';

// file은 이 실행의 원고 파일 경로 — 커버리지·인용 대조가 파일별로 일반화되어 있어
// onSubmit의 정합성 검사가 어느 원고에 대한 것인지 명시해야 한다.
export type SubmitContext = { turn: number; tools: ToolRecord[]; ledger: StageLedger; file: string };

// 수용이면 스테이지가 accept 값을 돌려주며 끝난다. 반려면 사유가 모델에게 돌아가고 계속된다.
export type SubmitOutcome<R> = { accept: R; message: string } | { reject: string[] };

export type AgentStageOptions<T, R> = {
  clients: LlmClients;
  stage: string;
  prompt: PhasePrompt;
  // 실행 단위 워크스페이스 — 스테이지가 이어받는다. 이전 산출물은 확정되어 읽기 전용이다.
  workspace: Workspace;
  // 원고 접근은 스테이지 속성 — 원고 대조가 설계상 없는 단계(compose류)는 끈다.
  manuscriptAccess: boolean;
  system: string;
  initial: string;
  search: SearchExecutor | null;
  deliverable: Deliverable;
  // 이 실행의 원고 파일 경로. SubmitContext.file로 전파된다.
  manuscriptFile: string;
  // 이전 단계에서 넘어온 도구 기록 — 열람 범위 검증이 단계를 넘어 이어진다.
  baseTools: ToolRecord[];
  // 원장을 남길 키와 누산기. 한 매니페스트 단계가 내부적으로 여러 라운드로 갈릴 때
  // 라운드마다 따로 남기면 같은 도구가 중복되고 화면의 단계 순서도 무너진다.
  ledgerKey?: string;
  ledger?: StageLedger;
  // 접수 시 산출물을 확정(헤더 부착·불변화)할지. 라운드가 이어지는 스테이지(계획 검수의
  // 수정 라운드)는 끄고, 소유 스테이지가 수렴 후 직접 확정한다. 기본 true.
  finalizeOnAccept?: boolean;
  onSubmit: (path: string, value: T, ctx: SubmitContext) => SubmitOutcome<R>;
};

const renderNotes = (notes: string[]): string =>
  notes.length === 0 ? '검사: 통과' : `검사 노트 ${notes.length}건:\n${notes.map((n) => `- ${n}`).join('\n')}`;

const pathOf = (use: { input: unknown }): string => {
  const input = (use.input ?? {}) as { path?: unknown };
  return typeof input.path === 'string' ? input.path : '';
};

export const runAgentStage = async <T, R>(
  ctx: RunContext,
  options: AgentStageOptions<T, R>,
): Promise<{ value: R; ledger: StageLedger }> => {
  const { clients, stage, prompt, search, deliverable, workspace, baseTools, onSubmit } = options;
  const ledger = options.ledger ?? emptyLedger();
  const ledgerKey = options.ledgerKey ?? stage;
  // 권한을 먼저 세운다 — 색인(가이드에 렌더)이 현재 권한을 반영해야 한다.
  workspace.setDeclaredOutputs(Object.keys(deliverable.outputs));
  workspace.setManuscriptAccess(options.manuscriptAccess);
  const tools = [...deliverableTools(deliverable), ...(search ? [SEARCH_TOOL] : [])];
  const system = [options.system, deliverableGuide(deliverable, workspace.index())].filter((s) => s.length > 0).join('\n\n');
  const messages: Anthropic.MessageParam[] = [{ role: 'user', content: options.initial }];

  for (let turn = 0; turn < TURN_CAP; turn++) {
    // step.do의 반환 타입은 Serializable로 좁혀져 있어 unknown을 품은 구조를 그대로 통과시키지
    // 못한다. 값은 실제로 JSON 왕복이 되므로 경계에서만 단언한다.
    const turnStep = (await ctx.step.do(`${stage}-${turn}`, LLM_STEP, async () => {
      const { value, cached } = await ctx.cached<TurnOutput>(`${stage}/turn/${turn}`, (usage) =>
        runTurn(clients, prompt, tools, system, messages, usage),
      );
      return { out: value, cached } as never;
    })) as unknown as { out: TurnOutput; cached: boolean };

    const out = turnStep.out;
    messages.push({ role: 'assistant', content: out.content as Anthropic.MessageParam['content'] });

    if (out.toolUses.length === 0) {
      // 텍스트만 있는 턴도 진행 기록에는 남긴다 — 모델이 어디서 머뭇거렸는지가 보인다.
      ledger.turns.push(turnNote(stage, turn, out.content, []));
      await ctx.ledger(`ledger/${ledgerKey}`, ledger);
      messages.push({ role: 'user', content: '도구를 호출하거나 제출 도구로 마무리하세요.' });
      continue;
    }

    const actions: string[] = [];
    const resultOf = new Map<string, string>();

    // 파일 연산 — 순수하므로 캐시 밖에서 즉시 적용한다. 산출물 저장이 일어나면 검사 노트를
    // 붙인다(scratch는 무검사). 원고 관찰 기록은 정합성 원장에 쌓인다.
    for (const use of out.toolUses) {
      const outcome = workspace.apply(use, turn);
      if (outcome === null) continue;
      const path = pathOf(use);
      const note =
        outcome.changed && Object.hasOwn(deliverable.outputs, path)
          ? `${outcome.message}\n${renderNotes(validateOutput(deliverable, path, workspace.file(path)).notes)}`
          : outcome.message;
      resultOf.set(use.id, note);
      if (outcome.record) ledger.tools.push(outcome.record);
      actions.push(workspace.summarize(use));
    }

    // 검색 — 비결정적이므로 실행 결과를 캐시한다. 이 턴에 검색이 없으면 스텝을 만들지
    // 않는다(분기는 캐시된 턴 출력의 순수 함수라 리플레이에 안전).
    const searchUses = out.toolUses.filter((use) => !resultOf.has(use.id) && use.name !== deliverable.submitName);
    const executed =
      searchUses.length === 0
        ? { results: [], records: [] }
        : ((await ctx.step.do(`${stage}-tools-${turn}`, async () => {
            const { value } = await ctx.cached(`${stage}/tools/${turn}`, () => executeSearches(searchUses, turn, search));
            return value as never;
          })) as unknown as Awaited<ReturnType<typeof executeSearches>>);
    for (const r of executed.results) resultOf.set(r.toolUseId, r.content);
    ledger.tools.push(...executed.records);

    // 제출 — 저장 검사와 같은 게이트를 통과한 값만 도메인 수용에 닿는다. 접수되면 그 자리에서
    // 확정된다: 자기 서술 헤더가 붙고 이후 수정이 봉인된다.
    const combinedTools = [...baseTools, ...ledger.tools];
    let accepted: R | undefined;
    for (const use of out.toolUses) {
      if (use.name !== deliverable.submitName || resultOf.has(use.id)) continue;
      const path = pathOf(use);
      const gate = validateOutput<T>(deliverable, path, workspace.file(path));
      if (gate.value === undefined) {
        ledger.events.push({ turn, kind: 'submit-rejected', detail: `검사 노트 ${gate.notes.length}건` });
        resultOf.set(use.id, renderRejection(gate.notes));
        actions.push(`${deliverable.submitName} ${path} → 반려 ${gate.notes.length}건`);
        continue;
      }
      const outcome = onSubmit(path, gate.value, { turn, tools: combinedTools, ledger, file: options.manuscriptFile });
      if ('reject' in outcome) {
        resultOf.set(use.id, renderRejection(outcome.reject));
        actions.push(`${deliverable.submitName} ${path} → 반려 ${outcome.reject.length}건`);
        continue;
      }
      if (options.finalizeOnAccept ?? true) {
        const spec = deliverable.outputs[path];
        workspace.finalize(path, finalizeHeader(spec, deliverable.label), spec.description);
      }
      accepted = outcome.accept;
      resultOf.set(use.id, outcome.message);
      actions.push(`${deliverable.submitName} ${path} → 접수`);
    }

    ledger.turns.push(turnNote(stage, turn, out.content, actions));

    messages.push({
      role: 'user',
      content: out.toolUses.map((use) => ({
        type: 'tool_result' as const,
        tool_use_id: use.id,
        content: resultOf.get(use.id) ?? '처리되지 않은 호출',
      })),
    });

    // 스테이지가 끝나면 작업 메모를 스냅샷으로 남긴다 — 무엇을 메모하며 일했는지의 사후 열람.
    if (accepted !== undefined) ledger.scratchFiles = workspace.scratchFiles();

    // 단계가 끝나야 원장이 남으면 도는 동안 무엇을 읽고 무엇이 걸렸는지 볼 수 없다.
    // 턴마다 덮어써 진행 중에도 최신 상태가 보이게 한다.
    await ctx.ledger(`ledger/${ledgerKey}`, ledger);

    if (accepted !== undefined) return { value: accepted, ledger };
  }

  throw new Error(`${stage}: 턴 백스톱(${TURN_CAP}) 초과`);
};
