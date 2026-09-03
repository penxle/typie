import { PRISM_VISIBILITY_MOTION } from '../../../@prism/lib/motion.ts';
import { COLUMN_GAP, COLUMN_WIDTH, GUTTER } from './margin-view.ts';
import type { MarginMode } from './margin-view.ts';

const clampProgress = (progress: number): number => Math.max(0, Math.min(1, progress));

export const marginInsets = (progress: number): { left: number; right: number } => {
  const value = clampProgress(progress);
  return { left: GUTTER * value, right: (COLUMN_WIDTH + COLUMN_GAP) * value };
};

export const lanePresentation = (progress: number): { opacity: number; scale: number } => {
  const value = clampProgress(progress);
  return {
    opacity: value,
    scale: PRISM_VISIBILITY_MOTION.hiddenScale + (1 - PRISM_VISIBILITY_MOTION.hiddenScale) * value,
  };
};

export const nextMarginReserved = (current: boolean, selectedRoundId: string | null, ready: boolean): boolean =>
  selectedRoundId === null ? false : ready ? true : current;

export const marginMotionTarget = (mode: MarginMode, idle: boolean, reserved: boolean, prepared: boolean): 0 | 1 =>
  mode === 'column' && !idle && reserved && prepared ? 1 : 0;

export const marginMotionDuration = (reduceMotion: boolean): number => (reduceMotion ? 0 : PRISM_VISIBILITY_MOTION.duration);

export type RoundSwapState =
  | { phase: 'idle' }
  | { phase: 'fading-out'; targetId: string }
  | { phase: 'waiting'; targetId: string }
  | { phase: 'preparing'; targetId: string; spinnerVisible: boolean }
  | { phase: 'fading-in'; targetId: string };

type RoundSwapInput = {
  selectedRoundId: string | null;
  presentedRoundId: string | null;
  loadedRoundId: string | null;
  visibilityProgress: number;
  prepared: boolean;
  failed: boolean;
};

type RoundSwapResolution = {
  state: RoundSwapState;
  replacePresented: boolean;
  restoreSelection: boolean;
  visibilityTarget: 0 | 1;
};

const resolve = (
  state: RoundSwapState,
  visibilityTarget: 0 | 1,
  replacePresented = false,
  restoreSelection = false,
): RoundSwapResolution => ({ state, visibilityTarget, replacePresented, restoreSelection });

const resolveIdle = (state: RoundSwapState): RoundSwapResolution => resolve(state.phase === 'idle' ? state : { phase: 'idle' }, 1);

export const resolveRoundSwap = (state: RoundSwapState, input: RoundSwapInput): RoundSwapResolution => {
  const { selectedRoundId, presentedRoundId, loadedRoundId, prepared, failed } = input;
  const visibility = clampProgress(input.visibilityProgress);

  // 첫 표시와 완전 닫힘은 컬럼 자체의 presentation이 소유한다. 이 상태는 보이는 회차끼리의 교체만 맡는다.
  if (selectedRoundId === null || presentedRoundId === null) return resolveIdle(state);

  if (presentedRoundId === selectedRoundId && state.phase === 'preparing' && state.targetId === selectedRoundId) {
    if (prepared) return resolve({ phase: 'fading-in', targetId: selectedRoundId }, 1);
    return resolve(state, 0);
  }

  if (presentedRoundId === selectedRoundId && state.phase === 'fading-in' && state.targetId === selectedRoundId) {
    if (visibility === 1) return resolveIdle(state);
    return resolve(state, 1);
  }

  // fade-out 중 원래 회차를 다시 고른 경우를 포함해, 현재 표시 회차가 목표면 그 자리에서 되돌린다.
  if (selectedRoundId === presentedRoundId) {
    if (visibility === 1) return resolveIdle(state);
    return resolve({ phase: 'fading-in', targetId: presentedRoundId }, 1);
  }

  // 실패한 결과로 표시 회차를 교체하지 않는다. 호출자는 선택만 기존 회차로 되돌리면 된다.
  if (failed && loadedRoundId !== selectedRoundId) {
    if (state.phase === 'fading-in' && state.targetId === presentedRoundId) return resolve(state, 1);
    return resolve({ phase: 'fading-in', targetId: presentedRoundId }, 1, false, true);
  }

  if (visibility > 0) {
    if (state.phase === 'fading-out' && state.targetId === selectedRoundId) return resolve(state, 0);
    return resolve({ phase: 'fading-out', targetId: selectedRoundId }, 0);
  }

  if (loadedRoundId !== selectedRoundId) {
    if (state.phase === 'waiting' && state.targetId === selectedRoundId) return resolve(state, 0);
    return resolve({ phase: 'waiting', targetId: selectedRoundId }, 0);
  }

  const spinnerVisible = state.phase === 'waiting' || (state.phase === 'preparing' && state.spinnerVisible);
  const next =
    state.phase === 'preparing' && state.targetId === selectedRoundId && state.spinnerVisible === spinnerVisible
      ? state
      : { phase: 'preparing' as const, targetId: selectedRoundId, spinnerVisible };
  return resolve(next, 0, presentedRoundId !== selectedRoundId);
};

export const visibleAreaCenterY = (
  viewportTop: number,
  viewportHeight: number,
  visibleArea: { topInset: number; bottomInset: number },
): number => {
  const topInset = Math.max(0, visibleArea.topInset);
  const visibleHeight = Math.max(0, viewportHeight - topInset - Math.max(0, visibleArea.bottomInset));
  return viewportTop + topInset + visibleHeight / 2;
};

export const contentMotionOffset = (target: 0 | 1, progress: number, shift: number): number => (target - clampProgress(progress)) * shift;

export const targetColumnLeft = (pageRight: number, progress: number): number => {
  const remaining = 1 - clampProgress(progress);
  return pageRight + COLUMN_GAP + ((GUTTER - (COLUMN_WIDTH + COLUMN_GAP)) * remaining) / 2;
};
