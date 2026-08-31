import { describe, expect, it } from 'vitest';
import { ContextBarVisibilityCoordinator, resolveContextBarSegmentRequest } from './editor-context-bar.svelte';
import type { ContextBarSegmentActivity } from './editor-context-bar.svelte';

const idle = (): ContextBarSegmentActivity => ({ transient: false, hovered: false, focused: false, holds: [] });

const transient = (): ContextBarSegmentActivity => ({ ...idle(), transient: true });

const engaged = (): ContextBarSegmentActivity => ({ ...idle(), hovered: true });

describe('editor context bar visibility', () => {
  it('counts transient, engagement, and named holds as visibility reasons', () => {
    expect(resolveContextBarSegmentRequest(idle())).toEqual({ visible: false, tone: 'transient' });
    expect(resolveContextBarSegmentRequest(transient())).toEqual({ visible: true, tone: 'transient' });
    expect(resolveContextBarSegmentRequest(engaged())).toEqual({ visible: true, tone: 'engaged' });
    expect(resolveContextBarSegmentRequest({ ...idle(), holds: ['overshoot'] })).toEqual({
      visible: true,
      tone: 'transient',
    });
  });

  it('unifies two visible segments regardless of whether their fade wings meet', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    expect(coordinator.resolve({ leading: transient(), viewControls: engaged() })).toEqual({
      unified: true,
      leading: { visible: true, tone: 'transient' },
      viewControls: { visible: true, tone: 'engaged' },
    });
  });

  it('retains both segments while either unified segment still has a visibility reason', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    coordinator.resolve({ leading: transient(), viewControls: engaged() });
    expect(coordinator.resolve({ leading: idle(), viewControls: engaged() })).toEqual({
      unified: true,
      leading: { visible: true, tone: 'transient' },
      viewControls: { visible: true, tone: 'engaged' },
    });
  });

  it('does not reveal a sibling until both segments have appeared together', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    expect(coordinator.resolve({ leading: transient(), viewControls: idle() })).toEqual({
      unified: false,
      leading: { visible: true, tone: 'transient' },
      viewControls: { visible: false, tone: 'transient' },
    });
  });

  it('dismisses a unified pair only after both segments have no visibility reason', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    coordinator.resolve({ leading: transient(), viewControls: engaged() });
    expect(
      coordinator.resolve({
        leading: idle(),
        viewControls: { ...idle(), holds: ['landmark'] },
      }),
    ).toEqual({
      unified: true,
      leading: { visible: true, tone: 'transient' },
      viewControls: { visible: true, tone: 'transient' },
    });

    expect(coordinator.resolve({ leading: idle(), viewControls: idle() })).toEqual({
      unified: false,
      leading: { visible: false, tone: 'transient' },
      viewControls: { visible: false, tone: 'transient' },
    });
  });
});
