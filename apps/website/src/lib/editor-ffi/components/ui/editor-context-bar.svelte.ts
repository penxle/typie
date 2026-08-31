import type { TransientVisibilityActivity, TransientVisibilityState } from './transient-visibility.svelte';

export const CONTEXT_BAR_TRANSIENT_VISIBLE_MS = 1500;
export const CONTEXT_BAR_FADE_IN_MS = 180;
export const CONTEXT_BAR_FADE_OUT_MS = 400;

export type ContextBarTone = 'transient' | 'engaged';

export type ContextBarSegmentActivity = TransientVisibilityActivity;

export type ContextBarSegmentPresentation = {
  visible: boolean;
  tone: ContextBarTone;
};

export type EditorContextBarSegmentState = TransientVisibilityState;

type ContextBarPresentationInput = {
  leading: ContextBarSegmentActivity;
  viewControls: ContextBarSegmentActivity;
};

export type ContextBarPresentation = {
  unified: boolean;
  leading: ContextBarSegmentPresentation;
  viewControls: ContextBarSegmentPresentation;
};

export function resolveContextBarSegmentRequest(activity: ContextBarSegmentActivity): ContextBarSegmentPresentation {
  const engaged = activity.hovered || activity.focused;
  return {
    visible: activity.transient || engaged || activity.holds.length > 0,
    tone: engaged ? 'engaged' : 'transient',
  };
}

export class ContextBarVisibilityCoordinator {
  #unified = false;

  resolve(input: ContextBarPresentationInput): ContextBarPresentation {
    const requested = {
      leading: resolveContextBarSegmentRequest(input.leading),
      viewControls: resolveContextBarSegmentRequest(input.viewControls),
    };

    if (!this.#unified && requested.leading.visible && requested.viewControls.visible) this.#unified = true;

    if (!requested.leading.visible && !requested.viewControls.visible) {
      this.#unified = false;
      return { unified: false, ...requested };
    }

    if (!this.#unified) return { unified: false, ...requested };

    return {
      unified: true,
      leading: requested.leading.visible ? requested.leading : { visible: true, tone: 'transient' },
      viewControls: requested.viewControls.visible ? requested.viewControls : { visible: true, tone: 'transient' },
    };
  }
}
