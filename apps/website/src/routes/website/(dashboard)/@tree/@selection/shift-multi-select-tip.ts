import { Tip } from '@typie/ui/notification';
import type { TreeState } from '../state.svelte';

const SHIFT_MULTI_SELECT_STREAK_REQUIRED = 3;
const SHIFT_MULTI_SELECT_STREAK_INTERVAL_MS = 3000;
const SHIFT_MULTI_SELECT_TIP_KEY = 'tree.shortcut.range-select';
const SHIFT_MULTI_SELECT_TIP_MESSAGE = '`Shift` 키를 누른 채 체크하면 중간에 있는 항목들이 모두 선택돼요.';

type ShiftMultiSelectTipTracker = {
  streak: number;
  lastSelectedAt: number;
  lastEntityId?: string;
};

const trackers = new WeakMap<TreeState, ShiftMultiSelectTipTracker>();

const getTracker = (treeState: TreeState) => {
  const tracker = trackers.get(treeState);
  if (tracker) {
    return tracker;
  }

  const initialized: ShiftMultiSelectTipTracker = {
    streak: 0,
    lastSelectedAt: 0,
    lastEntityId: undefined,
  };
  trackers.set(treeState, initialized);
  return initialized;
};

export const resetShiftMultiSelectTip = (treeState: TreeState) => {
  const tracker = getTracker(treeState);
  tracker.streak = 0;
  tracker.lastSelectedAt = 0;
  tracker.lastEntityId = undefined;
};

const shouldShowShiftMultiSelectTip = (treeState: TreeState, entityId: string, selectedCountBefore: number, selectedCountAfter: number) => {
  if (selectedCountAfter !== selectedCountBefore + 1) {
    resetShiftMultiSelectTip(treeState);
    return false;
  }

  const now = Date.now();
  const tracker = getTracker(treeState);
  const isConsecutive =
    tracker.lastSelectedAt > 0 &&
    now - tracker.lastSelectedAt <= SHIFT_MULTI_SELECT_STREAK_INTERVAL_MS &&
    tracker.lastEntityId !== entityId;

  tracker.streak = isConsecutive ? tracker.streak + 1 : 1;
  tracker.lastSelectedAt = now;
  tracker.lastEntityId = entityId;

  if (selectedCountBefore < 1) {
    return false;
  }

  return tracker.streak === SHIFT_MULTI_SELECT_STREAK_REQUIRED;
};

export const showShiftMultiSelectTipIfNeeded = (
  treeState: TreeState,
  entityId: string,
  selectedCountBefore: number,
  selectedCountAfter: number,
) => {
  if (!shouldShowShiftMultiSelectTip(treeState, entityId, selectedCountBefore, selectedCountAfter)) {
    return;
  }

  Tip.show(SHIFT_MULTI_SELECT_TIP_KEY, SHIFT_MULTI_SELECT_TIP_MESSAGE, { once: false });
};
