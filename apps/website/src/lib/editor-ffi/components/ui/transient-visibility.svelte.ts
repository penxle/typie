export type TransientVisibilityActivity = {
  transient: boolean;
  hovered: boolean;
  focused: boolean;
  holds: readonly string[];
};

export class TransientVisibilityState {
  #hideTimer: ReturnType<typeof setTimeout> | undefined;
  transient = $state(false);
  hovered = $state(false);
  focused = $state(false);
  holds = $state<string[]>([]);

  get activity(): TransientVisibilityActivity {
    return {
      transient: this.transient,
      hovered: this.hovered,
      focused: this.focused,
      holds: this.holds,
    };
  }

  get visible(): boolean {
    return this.transient || this.hovered || this.focused || this.holds.length > 0;
  }

  get engaged(): boolean {
    return this.hovered || this.focused;
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
