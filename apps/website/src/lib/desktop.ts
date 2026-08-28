import { browser } from '$app/environment';
import type { TabIcon, TypieDesktopBridge } from '@typie/lib/desktop';

export const desktop: TypieDesktopBridge | null = browser ? (window.typieDesktop ?? null) : null;

const tabIcons = new Map<symbol, TabIcon>();
let announced: TabIcon | null = null;
let scheduled = false;

const announceTabIcon = () => {
  scheduled = false;
  const next = [...tabIcons.values()].at(-1);
  if (!next || (next.icon === announced?.icon && next.color === announced?.color)) return;
  announced = next;
  desktop?.setTabIcon?.(next);
};

const scheduleTabIcon = () => {
  if (scheduled) return;
  scheduled = true;
  queueMicrotask(announceTabIcon);
};

export const setTabIcon = (key: symbol, icon: TabIcon | null) => {
  if (!desktop?.setTabIcon) return;
  if (icon) tabIcons.set(key, icon);
  else tabIcons.delete(key);
  scheduleTabIcon();
};
