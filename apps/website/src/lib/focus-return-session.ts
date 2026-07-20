export class FocusReturnSession {
  static capture(target: EventTarget | null): FocusReturnSession | null {
    if (typeof document === 'undefined' || !(target instanceof HTMLElement)) return null;
    if (target.ownerDocument !== document || !target.isConnected) return null;
    if (target === document.body || target === document.documentElement) return null;
    return new FocusReturnSession(target);
  }

  #target: HTMLElement | null;

  private constructor(target: HTMLElement) {
    this.#target = target;
  }

  #take(): HTMLElement | null {
    const target = this.#target;
    this.#target = null;
    return target;
  }

  restore(): boolean {
    const target = this.#take();
    if (!target || target.ownerDocument !== document || !target.isConnected) return false;

    try {
      target.focus({ preventScroll: true });
    } catch {
      return false;
    }

    return document.activeElement === target;
  }

  restoreIfFocusWithin(region: Node): boolean {
    if (typeof document === 'undefined' || !region.contains(document.activeElement)) {
      this.discard();
      return false;
    }
    return this.restore();
  }

  discard(): void {
    this.#target = null;
  }
}
