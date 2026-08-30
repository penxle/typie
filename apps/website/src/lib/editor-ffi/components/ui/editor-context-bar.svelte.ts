export const CONTEXT_BAR_TRANSIENT_VISIBLE_MS = 1500;
export const CONTEXT_BAR_FADE_IN_MS = 180;
export const CONTEXT_BAR_FADE_OUT_MS = 400;

export type ContextBarTone = 'transient' | 'engaged';

export type ContextBarSegmentActivity = {
  transient: boolean;
  hovered: boolean;
  focused: boolean;
  holds: readonly string[];
};

export type ContextBarSegmentPresentation = {
  visible: boolean;
  tone: ContextBarTone;
};

export class EditorContextBarSegmentState {
  #hideTimer: ReturnType<typeof setTimeout> | undefined;
  transient = $state(false);
  hovered = $state(false);
  focused = $state(false);
  holds = $state<string[]>([]);

  get activity(): ContextBarSegmentActivity {
    return {
      transient: this.transient,
      hovered: this.hovered,
      focused: this.focused,
      holds: this.holds,
    };
  }

  showTemporarily(durationMs: number): void {
    this.transient = true;
    clearTimeout(this.#hideTimer);
    this.#hideTimer = setTimeout(() => {
      this.transient = false;
      this.#hideTimer = undefined;
    }, durationMs);
  }

  hideTransient(): void {
    clearTimeout(this.#hideTimer);
    this.#hideTimer = undefined;
    this.transient = false;
  }

  setHovered(hovered: boolean): void {
    this.hovered = hovered;
  }

  setFocused(focused: boolean): void {
    this.focused = focused;
  }

  hold(reason: string): void {
    if (!this.holds.includes(reason)) this.holds = [...this.holds, reason];
  }

  release(reason: string): void {
    if (this.holds.includes(reason)) this.holds = this.holds.filter((hold) => hold !== reason);
  }

  destroy(): void {
    this.hideTransient();
    this.hovered = false;
    this.focused = false;
    this.holds = [];
  }
}

type ContextBarPresentationInput = {
  breadcrumb: ContextBarSegmentActivity;
  viewControls: ContextBarSegmentActivity;
};

export type ContextBarPresentation = {
  unified: boolean;
  breadcrumb: ContextBarSegmentPresentation;
  viewControls: ContextBarSegmentPresentation;
};

const clampUnit = (value: number): number => Math.min(1, Math.max(0, value));

export function smootherstep(value: number): number {
  const x = clampUnit(value);
  return x * x * x * (x * (x * 6 - 15) + 10);
}

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
      breadcrumb: resolveContextBarSegmentRequest(input.breadcrumb),
      viewControls: resolveContextBarSegmentRequest(input.viewControls),
    };

    if (!this.#unified && requested.breadcrumb.visible && requested.viewControls.visible) this.#unified = true;

    if (!requested.breadcrumb.visible && !requested.viewControls.visible) {
      this.#unified = false;
      return { unified: false, ...requested };
    }

    if (!this.#unified) return { unified: false, ...requested };

    return {
      unified: true,
      breadcrumb: requested.breadcrumb.visible ? requested.breadcrumb : { visible: true, tone: 'transient' },
      viewControls: requested.viewControls.visible ? requested.viewControls : { visible: true, tone: 'transient' },
    };
  }
}
