import { createFindRange } from '../../../core/text.ts';
import { checkFinding, checkQuote, coverageGaps, FILE_REJECT_MAX } from '../checks.ts';
import { FILE_STRENGTH_TOOL, fileFindingTool, GREP_TOOL, READ_TOOL, SUBMIT_REVIEW_TOOL } from '../contracts.ts';
import { renderEditorialPlanBlock, renderRejection } from '../render.ts';
import { runAgentStage, shapeRejection } from './agent-stage.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { StageLedger } from '../ledger.ts';
import type { AcceptedFinding, AcceptedStrength, EditorialFinding, EditorialPlan, EditorialStrength, ResolvedResearch } from '../types.ts';

export type ExecuteResult = {
  findings: AcceptedFinding[];
  strengths: AcceptedStrength[];
  discardedByAxis: Map<string, number>;
  ledger: StageLedger;
};

// 순차 통독 + 증분 제출. 커버리지 게이트가 종료를 지킨다.
export const runExecute = async (
  ctx: RunContext,
  client: Anthropic,
  charter: string,
  plan: EditorialPlan,
  research: ResolvedResearch,
): Promise<ExecuteResult> => {
  const content = ctx.document.content;
  const axisLabels = plan.axes.map((a) => a.label);
  const executeFindingTool = fileFindingTool(axisLabels);
  const findings: AcceptedFinding[] = [];
  const strengths: AcceptedStrength[] = [];
  const rejectCounts = new Map<string, number>();
  // 제출됐다가 유실된 지적의 축 — 지적 0건과 구별해 총평 cleared(무혐의)에서 제외한다.
  const discardedByAxis = new Map<string, number>();
  const findRange = createFindRange(content);

  const stage = await runAgentStage<true>(ctx, {
    client,
    stage: 'execute',
    prompt: ctx.prompts.execute,
    tools: [READ_TOOL, GREP_TOOL, executeFindingTool, FILE_STRENGTH_TOOL, SUBMIT_REVIEW_TOOL],
    system: [charter, '', renderEditorialPlanBlock(plan)].join('\n'),
    initial:
      '원고를 처음부터 순서대로 읽으세요. 읽다가 걸리면 그 자리에서 file_finding으로, 잘 작동하는 대목은 file_strength로 제출하세요. 끝까지 읽고 확인이 끝나면 submit_review로 마치세요.',
    search: null,
    baseTools: [],
    onSubmissions: (subs, turn, tools, ledger) => {
      const results: { toolUseId: string; content: string }[] = [];
      let done: true | undefined;
      for (const sub of subs) {
        if (sub.name === 'file_finding') {
          const shape = shapeRejection(executeFindingTool, sub.input);
          if (shape) {
            ledger.events.push({ turn, kind: 'finding-rejected', detail: `오형 제출: ${shape[1] ?? ''}`.slice(0, 120) });
            results.push({ toolUseId: sub.id, content: renderRejection(shape) });
            continue;
          }
          const finding = sub.input as EditorialFinding;
          const check = checkFinding(content, finding, tools, turn, axisLabels, research.boundaryRanges);
          if (check.accepted) {
            findings.push(check.accepted);
            results.push({ toolUseId: sub.id, content: `접수 (${findings.length}건째).` });
            continue;
          }
          const key = finding.quoteStart.slice(0, 20);
          const count = (rejectCounts.get(key) ?? 0) + 1;
          rejectCounts.set(key, count);
          for (const reason of check.reasons) ledger.events.push({ turn, kind: 'finding-rejected', detail: `${key} — ${reason}` });
          if (count > FILE_REJECT_MAX) {
            ledger.events.push({ turn, kind: 'finding-discarded', detail: `${finding.axis} — ${key}` });
            discardedByAxis.set(finding.axis, (discardedByAxis.get(finding.axis) ?? 0) + 1);
            results.push({ toolUseId: sub.id, content: '반려 상한 초과 — 이 지적은 폐기합니다. 다음으로 진행하세요.' });
          } else {
            results.push({ toolUseId: sub.id, content: renderRejection(check.reasons) });
          }
          continue;
        }
        if (sub.name === 'file_strength') {
          const s = sub.input as EditorialStrength;
          const range = findRange(s.quoteStart, s.quoteEnd, 0);
          const quoteOk = range !== null && checkQuote(content, tools, s.quoteStart).range !== null;
          if (range && quoteOk) {
            strengths.push({ ...s, matchStart: range.rangeStart, matchEnd: range.rangeEnd });
            results.push({ toolUseId: sub.id, content: '접수.' });
          } else {
            results.push({ toolUseId: sub.id, content: renderRejection(['강점 앵커가 원고에 없거나 열람 범위 밖']) });
          }
          continue;
        }
        if (sub.name === 'submit_review') {
          const gaps = coverageGaps(content.length, tools, research.boundaryRanges);
          if (gaps.length > 0) {
            results.push({
              toolUseId: sub.id,
              content: renderRejection([`미열람 구간이 남았습니다: ${gaps.map((g) => `${g.start}~${g.end}`).join(', ')}`]),
            });
          } else {
            done = true;
            results.push({ toolUseId: sub.id, content: '검토 종료.' });
          }
          continue;
        }
        results.push({ toolUseId: sub.id, content: '알 수 없는 제출' });
      }
      return { done, results };
    },
  });

  // 중앙 차단으로 제거된 지적 제출도 유실이다 — 축이 식별되면 해당 축만, 아니면 어느 축의
  // 발견이 사라졌는지 알 수 없으므로 cleared 판정에서 그 축을 가려낼 수 없음을 원장에 남긴다.
  for (const leak of stage.ledger.leaked) {
    if (leak.name !== 'file_finding') continue;
    const axis = (leak.input as { axis?: unknown })?.axis;
    if (typeof axis === 'string' && axisLabels.includes(axis)) {
      discardedByAxis.set(axis, (discardedByAxis.get(axis) ?? 0) + 1);
    }
  }

  return { findings, strengths, discardedByAxis, ledger: stage.ledger };
};
