import { checkFinding, coverageGaps, leakLineLint } from '../checks.ts';
import { findingSchema, LOCAL_AXES } from '../contracts.ts';
import { runAgentStage } from './agent-stage.ts';
import type { LlmClients } from '../../../core/worker/compat.ts';
import type { Deliverable } from '../../../core/worker/deliverable.ts';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { Workspace } from '../../../core/worker/workspace.ts';
import type { StageLedger } from '../ledger.ts';
import type { AcceptedFinding, EditorialFinding, ResolvedResearch } from '../types.ts';

type LocalDraft = { findings: EditorialFinding[] };

const PROOFREAD_PATH = 'output/proofread.yaml';

const LOCAL_DELIVERABLE: Deliverable = {
  label: '교열 결과',
  submitName: 'submit_review',
  submitDescription: '교열 종료를 선언하고 산출물을 확정한다. 열람이 본문 전체를 덮고 검사를 통과해야 접수된다.',
  outputs: {
    [PROOFREAD_PATH]: {
      schema: {
        type: 'object',
        properties: {
          findings: { type: 'array', description: '읽다가 걸린 문면 지적. 걸린 그 자리에서 적는다', items: findingSchema(LOCAL_AXES) },
        },
        required: ['findings'],
        additionalProperties: false,
      },
      lints: [leakLineLint],
      description: '문면 교열 결과 — 문장 결·원고 사고 지적',
    },
  },
};

// 문면 층위(문장 결·원고 사고)의 단독 소유자. 접근 방식은 회수와 무관함이 실측됐고
// (주입 6/13 vs read 5/13), 회수를 만드는 것은 층위 전용 범위다.
export const runLocal = async (
  ctx: RunContext,
  clients: LlmClients,
  workspace: Workspace,
  manuscriptFile: string,
  charter: string,
  research: ResolvedResearch,
  // 작품 검토가 접수한 앵커의 문자 범위 — 겹침은 차단하지 않고 알린다(같은 대목의 다른
  // 층위 지적은 정당한 경우가 실측된다).
  executeAnchors: { matchStart: number; matchEnd: number }[],
): Promise<{ findings: AcceptedFinding[]; ledger: StageLedger }> => {
  const content = ctx.document.content;
  let nudged = false;

  const stage = await runAgentStage<LocalDraft, AcceptedFinding[]>(ctx, {
    clients,
    stage: 'local',
    prompt: ctx.prompts.local,
    workspace,
    manuscriptAccess: true,
    system: charter,
    initial: `원고를 처음부터 끝까지 read로 통독하며 문면 층위를 검토하세요. 걸리면 그 자리에서 ${PROOFREAD_PATH}의 findings에 적고, 끝나면 submit_review로 마치세요.`,
    search: null,
    deliverable: LOCAL_DELIVERABLE,
    manuscriptFile,
    baseTools: [],
    onSubmit: (_path, value, { turn, tools, ledger, file }) => {
      const gaps = coverageGaps(content.length, tools, research.boundaryRanges, file);
      if (gaps.length > 0) {
        return { reject: [`미열람 구간이 남았습니다: ${gaps.map((g) => `${g.start}~${g.end}`).join(', ')}`] };
      }
      // 실측된 유실 모드(대조까지 하고 미제출) 조준 — 종료 전 정확히 한 번 되묻는다.
      if (!nudged) {
        nudged = true;
        return {
          reject: [
            '제출 전 확인: 대조로 확인했지만 파일에 적지 않은 후보가 남아 있으면 지금 findings에 추가하세요. 없으면 submit_review를 다시 호출해 마치세요.',
          ],
        };
      }
      const notes: string[] = [];
      const accepted: AcceptedFinding[] = [];
      for (const [i, finding] of value.findings.entries()) {
        const key = finding.quoteStart.slice(0, 20);
        const check = checkFinding(content, finding, tools, turn, LOCAL_AXES, research.boundaryRanges, file);
        if (check.accepted) {
          accepted.push(check.accepted);
          continue;
        }
        for (const reason of check.reasons) {
          ledger.events.push({ turn, kind: 'finding-rejected', detail: `${key} — ${reason}` });
          notes.push(`findings[${i}] (${key}…): ${reason} — 고치거나 항목을 삭제하세요`);
        }
      }
      if (notes.length > 0) return { reject: notes };

      // 겹침 소프트 경고 — 접수 메시지에 동봉할 뿐 차단하지 않는다.
      const warnings: string[] = [];
      for (const f of accepted) {
        const overlaps = executeAnchors.filter((a) => f.matchStart < a.matchEnd && a.matchStart < f.matchEnd);
        if (overlaps.length > 0) {
          ledger.events.push({ turn, kind: 'finding-overlap', detail: f.quoteStart.slice(0, 20) });
          warnings.push(`"${f.quoteStart.slice(0, 20)}…" — 작품 검토 지적과 겹칩니다. 다른 층위의 결함인지 확인하세요`);
        }
      }
      return { accept: accepted, message: [`교열 종료. 지적 ${accepted.length}건 접수.`, ...warnings].join('\n') };
    },
  });

  return { findings: stage.value, ledger: stage.ledger };
};
