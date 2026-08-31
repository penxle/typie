import { describe, expect, it } from 'vitest';
import { resolveSidebarNavigationDrag, resolveSidebarNavigationGeometry } from './sidebar-navigation-resize';

describe('resolveSidebarNavigationGeometry', () => {
  it('clamps the persisted clip between the full height and the measured minimum viewport boundary', () => {
    expect(resolveSidebarNavigationGeometry(170, 86, 120)).toEqual({ clip: 84, maxClip: 84 });
    expect(resolveSidebarNavigationGeometry(170, 86, -10)).toEqual({ clip: 0, maxClip: 84 });
  });

  it('clamps a persisted clip against changed item geometry', () => {
    expect(resolveSidebarNavigationGeometry(140, 86, 70)).toEqual({ clip: 54, maxClip: 54 });
  });

  it('normalizes invalid measurements to a safe collapsed geometry', () => {
    expect(resolveSidebarNavigationGeometry(NaN, -20, Infinity)).toEqual({ clip: 0, maxClip: 0 });
  });
});

describe('resolveSidebarNavigationDrag', () => {
  it('applies pointer movement from the effective starting clip and clamps both directions', () => {
    const session = { startClip: 54, startY: 100 };

    expect(resolveSidebarNavigationDrag(session, 60, 54)).toEqual({ clip: 54, clipChanged: false });
    expect(resolveSidebarNavigationDrag(session, 200, 54)).toEqual({ clip: 0, clipChanged: true });
    expect(resolveSidebarNavigationDrag(session, 120, 54)).toEqual({ clip: 34, clipChanged: true });
  });

  it('does not persist an effective clip produced by a passive geometry clamp on click', () => {
    const geometry = resolveSidebarNavigationGeometry(140, 86, 70);

    expect(resolveSidebarNavigationDrag({ startClip: geometry.clip, startY: 100 }, 100, geometry.maxClip)).toEqual({
      clip: 54,
      clipChanged: false,
    });
  });
});
