import { comma } from '@typie/ui/utils';
import type { Dayjs } from 'dayjs';

export const GOAL_OVER_RATIO = 1.1;
export const GOAL_EXCESS_RATIO = 1.25;

export type GoalColorState = 'under' | 'achieved' | 'over' | 'excess';

export const goalColorState = (current: number, target: number): GoalColorState => {
  if (current < target) {
    return 'under';
  }

  if (current <= target * GOAL_OVER_RATIO) {
    return 'achieved';
  }

  if (current <= target * GOAL_EXCESS_RATIO) {
    return 'over';
  }

  return 'excess';
};

export const requiredToday = (current: number, target: number, dueAt: Dayjs, today: Dayjs): number => {
  const remaining = target - current;
  if (remaining <= 0) {
    return 0;
  }

  const daysLeft = Math.max(1, dueAt.startOf('day').diff(today.startOf('day'), 'day') + 1);
  return Math.ceil(remaining / daysLeft);
};

export const timeFraction = (createdAt: Dayjs, dueAt: Dayjs, now: Dayjs): number => {
  const start = createdAt.startOf('day');
  const end = dueAt.startOf('day').add(1, 'day');
  const total = end.diff(start);

  if (total <= 0) {
    return 1;
  }

  return Math.min(1, Math.max(0, now.diff(start) / total));
};

export const dDayLabel = (dueAt: Dayjs, today: Dayjs): string => {
  const diff = dueAt.startOf('day').diff(today.startOf('day'), 'day');

  if (diff === 0) {
    return 'D-DAY';
  }

  return diff > 0 ? `D-${diff}` : `D+${-diff}`;
};

export const dueStatus = (
  current: number,
  target: number,
  dueAt: Dayjs,
  today: Dayjs,
  variant: 'full' | 'compact',
): { label: string; warning: boolean } | null => {
  const state = goalColorState(current, target);
  const duePassed = dueAt.startOf('day').isBefore(today.startOf('day'));

  if (state !== 'under' && duePassed) {
    return null;
  }

  const required = requiredToday(current, target, dueAt, today);
  const warning = state === 'under' && duePassed;

  let suffix = '';
  if (required > 0) {
    if (warning) {
      suffix = ` · ${comma(required)}자 남음`;
    } else {
      suffix = variant === 'full' ? ` · 오늘 ${comma(required)}자 필요` : ` · ${comma(required)}자`;
    }
  }

  return { label: `${dDayLabel(dueAt, today)}${suffix}`, warning };
};

export const pickGoalSource = <G extends object>(
  entity: {
    id: string;
    goal?: G | null;
    ancestors: readonly { id: string; goal?: G | null; node: { __typename: string; characterCount?: number } }[];
  },
  ownCurrent: number,
): { goal: G; current: number; isFolder: boolean; entityId: string } | null => {
  if (entity.goal) {
    return { goal: entity.goal, current: ownCurrent, isFolder: false, entityId: entity.id };
  }

  const ancestor = entity.ancestors.findLast((a) => a.goal);
  const ancestorCurrent = ancestor?.node.__typename === 'Folder' ? ancestor.node.characterCount : undefined;

  if (ancestorCurrent !== undefined && ancestor?.goal) {
    return { goal: ancestor.goal, current: ancestorCurrent, isFolder: true, entityId: ancestor.id };
  }

  return null;
};
