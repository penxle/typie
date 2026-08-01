import { createFindRange } from '../../../core/text.ts';
import { checkFinding, checkQuote, coverageGaps, leakLineLint } from '../checks.ts';
import { findingSchema, STRENGTH_SCHEMA } from '../contracts.ts';
import { renderEditorialPlanBlock } from '../render.ts';
import { runAgentStage } from './agent-stage.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { Deliverable } from '../../../core/worker/deliverable.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { Workspace } from '../../../core/worker/workspace.ts';
import type { StageLedger } from '../ledger.ts';
import type { AcceptedFinding, AcceptedStrength, EditorialFinding, EditorialPlan, EditorialStrength, ResolvedResearch } from '../types.ts';

export type ExecuteResult = {
  findings: AcceptedFinding[];
  strengths: AcceptedStrength[];
  discardedByAxis: Map<string, number>;
  ledger: StageLedger;
};

type ReviewDraft = { findings: EditorialFinding[]; strengths: EditorialStrength[] };
type ExecuteAccepted = Omit<ExecuteResult, 'ledger'>;

const REVIEW_PATH = 'output/review.yaml';

// 순차 통독 + 산출물 누적. 커버리지 게이트가 종료를 지킨다.
export const runExecute = async (
  ctx: RunContext,
  clients: LlmClients,
  workspace: Workspace,
  manuscriptFile: string,
  charter: string,
  plan: EditorialPlan,
  research: ResolvedResearch,
): Promise<ExecuteResult> => {
  const content = ctx.document.content;
  const axisLabels = plan.axes.map((a) => a.label);
  const findRange = createFindRange(content);

  const deliverable: Deliverable = {
    label: '검토 결과',
    submitName: 'submit_review',
    submitDescription: '검토 종료를 선언하고 산출물을 확정한다. 열람이 본문 전체를 덮고 검사를 통과해야 접수된다.',
    outputs: {
      [REVIEW_PATH]: {
        schema: {
          type: 'object',
          properties: {
            findings: { type: 'array', description: '읽다가 걸린 지적. 걸린 그 자리에서 적는다', items: findingSchema(axisLabels) },
            strengths: { type: 'array', description: '잘 작동하는 대목. 조언·확장 제안 금지 — 총평의 재료다', items: STRENGTH_SCHEMA },
          },
          required: ['findings', 'strengths'],
          additionalProperties: false,
        },
        lints: [leakLineLint],
        description: '작품 검토 결과 — 계획 축 기반 지적과 강점',
      },
    },
  };

  // 제출 시점에 반려된 적 있는 지적의 축 — 최종 접수에서 빠지면 유실로 계산해, 지적 0건과
  // 구별하고 총평 cleared(무혐의)에서 그 축을 제외한다.
  const failedByKey = new Map<string, string>();

  const stage = await runAgentStage<ReviewDraft, ExecuteAccepted>(ctx, {
    clients,
    stage: 'execute',
    prompt: ctx.prompts.execute,
    workspace,
    manuscriptAccess: true,
    system: [charter, '', renderEditorialPlanBlock(plan)].join('\n'),
    initial: `원고를 처음부터 순서대로 읽으세요. 읽다가 걸린 지적은 그 자리에서 ${REVIEW_PATH}의 findings에, 잘 작동하는 대목은 strengths에 적으세요. 끝까지 읽고 확인이 끝나면 submit_review로 마치세요.`,
    search: null,
    deliverable,
    manuscriptFile,
    baseTools: [],
    onSubmit: (_path, value, { turn, tools, ledger, file }) => {
      const gaps = coverageGaps(content.length, tools, research.boundaryRanges, file);
      if (gaps.length > 0) {
        return { reject: [`미열람 구간이 남았습니다: ${gaps.map((g) => `${g.start}~${g.end}`).join(', ')}`] };
      }
      // 실패 항목은 고치거나 삭제하게 한다. 반려 상한·강제 폐기는 파일 흐름에 없다 —
      // 항목이 파일에 보이므로 처분은 모델의 편집이고, TURN_CAP이 방지선이다.
      const notes: string[] = [];
      const findings: AcceptedFinding[] = [];
      for (const [i, finding] of value.findings.entries()) {
        const key = finding.quoteStart.slice(0, 20);
        const check = checkFinding(content, finding, tools, turn, axisLabels, research.boundaryRanges, file);
        if (check.accepted) {
          findings.push(check.accepted);
          continue;
        }
        failedByKey.set(key, finding.axis);
        for (const reason of check.reasons) {
          ledger.events.push({ turn, kind: 'finding-rejected', detail: `${key} — ${reason}` });
          notes.push(`findings[${i}] (${key}…): ${reason} — 고치거나 항목을 삭제하세요`);
        }
      }
      const strengths: AcceptedStrength[] = [];
      for (const [i, s] of value.strengths.entries()) {
        const range = findRange(s.quoteStart, s.quoteEnd, 0);
        const quoteOk = range !== null && checkQuote(content, tools, s.quoteStart, file).range !== null;
        if (range && quoteOk) {
          strengths.push({ ...s, matchStart: range.rangeStart, matchEnd: range.rangeEnd });
        } else {
          notes.push(`strengths[${i}]: 앵커가 원고에 없거나 열람 범위 밖 — 고치거나 항목을 삭제하세요`);
        }
      }
      if (notes.length > 0) return { reject: notes };

      const acceptedKeys = new Set(findings.map((f) => f.quoteStart.slice(0, 20)));
      const discardedByAxis = new Map<string, number>();
      for (const [key, axis] of failedByKey) {
        if (acceptedKeys.has(key)) continue;
        ledger.events.push({ turn, kind: 'finding-discarded', detail: `${axis} — ${key}` });
        discardedByAxis.set(axis, (discardedByAxis.get(axis) ?? 0) + 1);
      }
      return {
        accept: { findings, strengths, discardedByAxis },
        message: `검토 종료. 지적 ${findings.length}건, 강점 ${strengths.length}건 접수.`,
      };
    },
  });

  return { ...stage.value, ledger: stage.ledger };
};
