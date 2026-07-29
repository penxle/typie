import type { EvaluationSpec } from '../../../../core/contracts.ts';

export type LockReason = 'round-inactive' | 'submitted' | null;

// 라운드를 닫으면 받아 놓은 임시저장도 즉시 잠긴다. 제출된 판정은 되돌리지 않는다.
export const judgmentLock = (round: { active: boolean }, judgment: { draft: boolean }): LockReason => {
  if (!round.active) return 'round-inactive';
  if (!judgment.draft) return 'submitted';
  return null;
};

export const LOCK_MESSAGE: Record<Exclude<LockReason, null>, string> = {
  'round-inactive': '이 라운드는 닫혀 있어 저장할 수 없습니다.',
  submitted: '이미 제출한 판정입니다.',
};

// 화면·검증이 쓰는 현재 단계 인덱스. 저장된 stage가 범위를 벗어나면 마지막 단계로 죈다.
export const stageIndexOf = (evaluation: Pick<EvaluationSpec, 'stages'>, judgment: { stage: number }): number =>
  Math.max(0, Math.min(judgment.stage, evaluation.stages.length - 1));

// 확정된 단계가 있으면 반납할 수 없다 — 반납은 확정된 답을 버린다.
export const releasable = (judgment: { stage: number }): boolean => judgment.stage === 0;
