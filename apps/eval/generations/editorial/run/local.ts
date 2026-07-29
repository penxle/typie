import { checkFinding, coverageGaps, FILE_REJECT_MAX } from '../checks.ts';
import { fileFindingTool, GREP_TOOL, LOCAL_AXES, READ_TOOL, SUBMIT_REVIEW_TOOL } from '../contracts.ts';
import { renderRejection } from '../render.ts';
import { runAgentStage, shapeRejection } from './agent-stage.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { RunContext } from '../../../core/worker/run-contracts.ts';
import type { StageLedger } from '../ledger.ts';
import type { AcceptedFinding, EditorialFinding, ResolvedResearch } from '../types.ts';

// 문면 층위(문장 결·원고 사고)의 단독 소유자. 접근 방식은 회수와 무관함이 실측됐고
// (주입 6/13 vs read 5/13), 회수를 만드는 것은 층위 전용 범위다.
export const runLocal = async (
  ctx: RunContext,
  client: Anthropic,
  charter: string,
  research: ResolvedResearch,
): Promise<{ findings: AcceptedFinding[]; ledger: StageLedger }> => {
  const content = ctx.document.content;
  const findings: AcceptedFinding[] = [];
  const localFindingTool = fileFindingTool(LOCAL_AXES);
  const localRejects = new Map<string, number>();
  let nudged = false;

  const stage = await runAgentStage<true>(ctx, {
    client,
    stage: 'local',
    prompt: ctx.prompts.local,
    tools: [READ_TOOL, GREP_TOOL, localFindingTool, SUBMIT_REVIEW_TOOL],
    system: charter,
    initial:
      '원고를 처음부터 끝까지 read로 통독하며 문면 층위를 검토하세요. 걸리면 그 자리에서 file_finding으로 제출하고, 끝나면 submit_review로 마치세요.',
    search: null,
    baseTools: [],
    onSubmissions: (subs, turn, tools, ledger) => {
      const results: { toolUseId: string; content: string }[] = [];
      let done: true | undefined;
      for (const sub of subs) {
        if (sub.name === 'file_finding') {
          const shape = shapeRejection(localFindingTool, sub.input);
          if (shape) {
            ledger.events.push({ turn, kind: 'finding-rejected', detail: `오형 제출: ${shape[1] ?? ''}`.slice(0, 120) });
            results.push({ toolUseId: sub.id, content: renderRejection(shape) });
            continue;
          }
          const finding = sub.input as EditorialFinding;
          const check = checkFinding(content, finding, tools, turn, LOCAL_AXES, research.boundaryRanges);
          if (check.accepted) {
            findings.push(check.accepted);
            results.push({ toolUseId: sub.id, content: '접수.' });
            continue;
          }
          const key = finding.quoteStart.slice(0, 20);
          const count = (localRejects.get(key) ?? 0) + 1;
          localRejects.set(key, count);
          for (const reason of check.reasons) ledger.events.push({ turn, kind: 'finding-rejected', detail: `${key} — ${reason}` });
          if (count > FILE_REJECT_MAX) {
            ledger.events.push({ turn, kind: 'finding-discarded', detail: key });
            results.push({ toolUseId: sub.id, content: '반려 상한 초과 — 이 지적은 폐기합니다. 다음으로 진행하세요.' });
          } else {
            results.push({ toolUseId: sub.id, content: renderRejection(check.reasons) });
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
            continue;
          }
          // 실측된 유실 모드(대조까지 하고 미제출) 조준 — 종료 전 정확히 한 번 되묻는다.
          if (!nudged) {
            nudged = true;
            results.push({
              toolUseId: sub.id,
              content:
                '제출 전 확인: 대조로 확인했지만 제출하지 않은 후보가 남아 있으면 지금 file_finding으로 제출하세요. 없으면 submit_review를 다시 호출해 마치세요.',
            });
            continue;
          }
          done = true;
          results.push({ toolUseId: sub.id, content: '검토 종료.' });
          continue;
        }
        results.push({ toolUseId: sub.id, content: '알 수 없는 제출' });
      }
      return { done, results };
    },
  });

  return { findings, ledger: stage.ledger };
};
