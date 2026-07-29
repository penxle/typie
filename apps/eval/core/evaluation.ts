import type { EvaluationSpec, EvaluationStage, FieldSpec, ItemMatchTarget, ItemTargetSpec } from './contracts.ts';

export type JudgedItem = { id: string } & ItemMatchTarget;

// 단계를 가로질러 찾는다 — 항목에 컨트롤이 걸리는지는 소유 단계와 무관하다.
export const targetFor = (evaluation: Pick<EvaluationSpec, 'stages'>, item: ItemMatchTarget): ItemTargetSpec | null =>
  evaluation.stages.flatMap((s) => s.items).find((target) => target.match(item)) ?? null;

// 완결·저장 판정은 단계 스코프다 — 현재 단계의 필드만 클라이언트에서 받는다.
export const stageTargetFor = (stage: Pick<EvaluationStage, 'items'>, item: ItemMatchTarget): ItemTargetSpec | null =>
  stage.items.find((target) => target.match(item)) ?? null;

// null은 '아직 답하지 않음'이며 통과와 구별되어야 한다. 기본값을 통과로 두면 평가자가
// 보지도 않은 항목이 전부 합격으로 집계된다.
export const missingFields = (fields: FieldSpec[], payload: Record<string, unknown>): string[] =>
  fields.filter((field) => field.required && field.sanitize(payload[field.key]) === null).map((field) => field.key);

export const judgmentGaps = (
  stage: Pick<EvaluationStage, 'run' | 'items'>,
  items: JudgedItem[],
  runPayload: Record<string, unknown>,
  itemPayloads: Record<string, Record<string, unknown>>,
): { run: string[]; items: { itemId: string; missing: string[] }[] } => {
  const gaps: { itemId: string; missing: string[] }[] = [];
  for (const item of items) {
    const target = stageTargetFor(stage, item);
    if (!target) continue;
    const missing = missingFields(target.fields, itemPayloads[item.id] ?? {});
    if (missing.length > 0) gaps.push({ itemId: item.id, missing });
  }
  return { run: missingFields(stage.run, runPayload), items: gaps };
};

export const isJudgmentComplete = (
  stage: Pick<EvaluationStage, 'run' | 'items'>,
  items: JudgedItem[],
  runPayload: Record<string, unknown>,
  itemPayloads: Record<string, Record<string, unknown>>,
): boolean => {
  const gaps = judgmentGaps(stage, items, runPayload, itemPayloads);
  return gaps.run.length === 0 && gaps.items.length === 0;
};

// 선언된 필드만 남기고 각자의 sanitize를 통과시킨다. 미답은 null로 명시된다 — 키가 사라지면
// "답하지 않음"과 "필드가 없었음"이 구별되지 않는다.
export const sanitizePayload = (fields: FieldSpec[], raw: Record<string, unknown>): Record<string, unknown> =>
  Object.fromEntries(fields.map((field) => [field.key, field.sanitize(raw[field.key])]));
