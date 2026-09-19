export class EditorLocalEditCoordinator {
  #lastAcceptedRequest = 0;
  #lastAppliedRequest = 0;
  #directMutationPending = false;
  #stops = $state(0);
  #input: { pending: () => boolean; finalize: () => void } | undefined;
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- synchronous admission/application notifications
  readonly #listeners = new Set<() => void>();

  get accepting(): boolean {
    return this.#stops === 0;
  }

  get queued(): boolean {
    return this.#lastAcceptedRequest > this.#lastAppliedRequest || this.#directMutationPending;
  }

  get pending(): boolean {
    return this.queued || (this.#input?.pending() ?? false);
  }

  accept(requestId: number): void {
    this.#lastAcceptedRequest = requestId;
  }

  apply(requestId: number): void {
    this.#lastAppliedRequest = Math.max(this.#lastAppliedRequest, requestId);
  }

  recordDirectMutation(): void {
    this.#directMutationPending = true;
  }

  beginTick(): boolean {
    const pending = this.queued;
    // Work admitted by tick callbacks belongs to the next application boundary.
    this.#directMutationPending = false;
    return pending;
  }

  stop(): () => void {
    this.#stops++;
    let released = false;
    return () => {
      if (released) return;
      released = true;
      this.#stops--;
    };
  }

  registerInput(input: { pending: () => boolean; finalize: () => void }): () => void {
    this.#input = input;
    this.notify();
    return () => {
      if (this.#input !== input) return;
      this.#input = undefined;
      this.notify();
    };
  }

  finalizeInput(): void {
    this.#input?.finalize();
  }

  onChange(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  notify(): void {
    for (const listener of this.#listeners) listener();
  }

  clear(): void {
    this.#listeners.clear();
    this.#input = undefined;
  }
}
