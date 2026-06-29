export type Partition = (blob: Uint8Array) => { ready: Uint8Array; blocked: Uint8Array };
export type ApplyReady = (ready: Uint8Array) => void;

type GapBufferOpts = {
  partition: Partition;
  apply: ApplyReady;
  onStuck: (blockedBlobs: Uint8Array[]) => void;
  stuckThreshold?: number;
};

export class GapBuffer {
  readonly #partition: Partition;
  readonly #apply: ApplyReady;
  readonly #onStuck: (blockedBlobs: Uint8Array[]) => void;
  readonly #stuckThreshold: number;
  #buffered: Uint8Array[] = [];
  #noProgress = 0;

  constructor(opts: GapBufferOpts) {
    this.#partition = opts.partition;
    this.#apply = opts.apply;
    this.#onStuck = opts.onStuck;
    this.#stuckThreshold = opts.stuckThreshold ?? 5;
  }

  #drain(): void {
    let progress = true;
    while (progress) {
      progress = false;
      const next: Uint8Array[] = [];
      for (const blob of this.#buffered) {
        const { ready, blocked } = this.#partition(blob);
        if (ready.length > 0) {
          this.#apply(ready);
          progress = true;
        }
        if (blocked.length > 0) next.push(blocked);
      }
      this.#buffered = next;
    }
    if (this.#buffered.length === 0) {
      this.#noProgress = 0;
      return;
    }
    this.#noProgress += 1;
    if (this.#noProgress >= this.#stuckThreshold) {
      const stuck = this.#buffered;
      this.#buffered = [];
      this.#noProgress = 0;
      this.#onStuck(stuck);
    }
  }

  ingest(blob: Uint8Array): void {
    this.#buffered.push(blob);
    this.#drain();
  }
}
