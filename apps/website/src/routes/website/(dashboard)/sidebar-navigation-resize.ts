import { clamp } from '@typie/ui/utils';

export type SidebarNavigationResizeSession = {
  startClip: number;
  startY: number;
};

const nonnegativeFinite = (value: number): number => (Number.isFinite(value) ? Math.max(0, value) : 0);

export const resolveSidebarNavigationGeometry = (intrinsicHeight: number, minimumHeight: number, requestedClip: number) => {
  const intrinsic = nonnegativeFinite(intrinsicHeight);
  const minimum = clamp(nonnegativeFinite(minimumHeight), 0, intrinsic);
  const maxClip = intrinsic - minimum;
  const clip = clamp(nonnegativeFinite(requestedClip), 0, maxClip);

  return { clip, maxClip };
};

export const resolveSidebarNavigationDrag = (session: SidebarNavigationResizeSession, pointerY: number, maximumClip: number) => {
  const maxClip = nonnegativeFinite(maximumClip);
  const startClip = clamp(nonnegativeFinite(session.startClip), 0, maxClip);
  if (!Number.isFinite(session.startY) || !Number.isFinite(pointerY)) return { clip: startClip, clipChanged: false };
  const clip = clamp(startClip - (pointerY - session.startY), 0, maxClip);
  return { clip, clipChanged: clip !== startClip };
};
