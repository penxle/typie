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

export const resolvePresentedRoundId = (selectedRoundId: string | null, lastRoundId: string | null, progress: number): string | null =>
  selectedRoundId ?? (clampProgress(progress) > 0 ? lastRoundId : null);

export const nextMarginReserved = (current: boolean, selectedRoundId: string | null, ready: boolean): boolean =>
  selectedRoundId === null ? false : ready ? true : current;

export const marginMotionTarget = (mode: MarginMode, idle: boolean, reserved: boolean, prepared: boolean): 0 | 1 =>
  mode === 'column' && !idle && reserved && prepared ? 1 : 0;

export const marginMotionDuration = (reduceMotion: boolean): number => (reduceMotion ? 0 : PRISM_VISIBILITY_MOTION.duration);

export const targetColumnLeft = (pageRight: number, progress: number): number => {
  const remaining = 1 - clampProgress(progress);
  return pageRight + COLUMN_GAP + ((GUTTER - (COLUMN_WIDTH + COLUMN_GAP)) * remaining) / 2;
};
