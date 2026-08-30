import { describe, expect, it } from 'vitest';
import { ContextBarVisibilityCoordinator, resolveContextBarSegmentRequest, smootherstep } from './editor-context-bar.svelte';
import type { ContextBarSegmentActivity } from './editor-context-bar.svelte';

const idle = (): ContextBarSegmentActivity => ({ transient: false, hovered: false, focused: false, holds: [] });

const transient = (): ContextBarSegmentActivity => ({ ...idle(), transient: true });

const engaged = (): ContextBarSegmentActivity => ({ ...idle(), hovered: true });

describe('editor context bar surface', () => {
  it('uses smootherstep for surface fades', () => {
    expect(smootherstep(0)).toBe(0);
    expect(smootherstep(0.25)).toBeCloseTo(0.103515625);
    expect(smootherstep(0.5)).toBe(0.5);
    expect(smootherstep(0.75)).toBeCloseTo(0.896484375);
    expect(smootherstep(1)).toBe(1);
  });
});

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

    expect(coordinator.resolve({ breadcrumb: transient(), viewControls: engaged() })).toEqual({
      unified: true,
      breadcrumb: { visible: true, tone: 'transient' },
      viewControls: { visible: true, tone: 'engaged' },
    });
  });

  it('retains both segments while either unified segment still has a visibility reason', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    coordinator.resolve({ breadcrumb: transient(), viewControls: engaged() });
    expect(coordinator.resolve({ breadcrumb: idle(), viewControls: engaged() })).toEqual({
      unified: true,
      breadcrumb: { visible: true, tone: 'transient' },
      viewControls: { visible: true, tone: 'engaged' },
    });
  });

  it('does not reveal a sibling until both segments have appeared together', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    expect(coordinator.resolve({ breadcrumb: transient(), viewControls: idle() })).toEqual({
      unified: false,
      breadcrumb: { visible: true, tone: 'transient' },
      viewControls: { visible: false, tone: 'transient' },
    });
  });

  it('dismisses a unified pair only after both segments have no visibility reason', () => {
    const coordinator = new ContextBarVisibilityCoordinator();

    coordinator.resolve({ breadcrumb: transient(), viewControls: engaged() });
    expect(
      coordinator.resolve({
        breadcrumb: idle(),
        viewControls: { ...idle(), holds: ['landmark'] },
      }),
    ).toEqual({
      unified: true,
      breadcrumb: { visible: true, tone: 'transient' },
      viewControls: { visible: true, tone: 'transient' },
    });

    expect(coordinator.resolve({ breadcrumb: idle(), viewControls: idle() })).toEqual({
      unified: false,
      breadcrumb: { visible: false, tone: 'transient' },
      viewControls: { visible: false, tone: 'transient' },
    });
  });
});
