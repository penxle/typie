import { createFindRange } from './text.ts';
import type { Plan } from './analysis-types.ts';

// 계획의 기계 검증. 산문 계약(축 3~6, 인용 실재)은 이 프로그램에서 한 번도 스스로
// 지켜진 적이 없다 — 앵커처럼 코드가 대조하고, 위반은 검수·재시도의 입력이 된다.
export const AXIS_MIN = 3;
export const AXIS_MAX = 6;
export const PROTECTED_MAX = 8;

export type PlanCheck = {
  // 미실재 인용을 걷어내고 상한을 자른 결과. 후속 단계는 이것을 쓴다.
  plan: Plan;
  // 축 수 위반은 걷어내서 고칠 수 있는 것이 아니라 계획을 다시 받아야 하는 결함이다.
  axisCountOk: boolean;
  // 사람이 읽을 위반 목록. 검수 입력과 진단 기록에 그대로 실린다.
  notes: string[];
};

export const checkPlan = (content: string, plan: Plan): PlanCheck => {
  const findRange = createFindRange(content);
  const resolves = (quote: string): boolean => quote.trim().length > 0 && findRange(quote, quote, 0) !== null;
  const notes: string[] = [];

  const protectedKept: Plan['protected'] = [];
  for (const p of plan.protected) {
    const kept: string[] = [];
    for (const q of p.evidence) {
      if (resolves(q)) kept.push(q);
      else notes.push(`보호 "${p.technique}" — 인용이 원고에 없음: ${q.slice(0, 40)}`);
    }
    if (kept.length === 0) {
      notes.push(`보호 "${p.technique}" — 실재하는 근거가 없어 등재 취소`);
      continue;
    }
    protectedKept.push({ ...p, evidence: kept });
  }
  if (protectedKept.length > PROTECTED_MAX) {
    notes.push(`보호 ${protectedKept.length}개 — 상한(${PROTECTED_MAX}) 초과, 뒤에서부터 잘라냄`);
    protectedKept.length = PROTECTED_MAX;
  }

  // 축은 근거가 비어도 제거하지 않는다 — 제거하면 축 수 계약이 조용히 깨지고,
  // 근거 없는 축의 처분(weak-axis)은 검수의 판단이다. 여기서는 사실만 남긴다.
  const axes = plan.axes.map((a) => {
    const kept: string[] = [];
    for (const q of a.evidence) {
      if (resolves(q)) kept.push(q);
      else notes.push(`축 "${a.label}" — 인용이 원고에 없음: ${q.slice(0, 40)}`);
    }
    if (kept.length === 0) notes.push(`축 "${a.label}" — 실재하는 근거가 없음`);
    return { ...a, evidence: kept };
  });

  const axisCountOk = axes.length >= AXIS_MIN && axes.length <= AXIS_MAX;
  if (!axisCountOk) notes.push(`축 ${axes.length}개 — 계약(${AXIS_MIN}~${AXIS_MAX}) 위반`);

  return { plan: { ...plan, protected: protectedKept, axes }, axisCountOk, notes };
};
