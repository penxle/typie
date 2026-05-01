import type { DebugEvent, TimelineEntry } from './types';

const RING_CAP = 500;

export class DebugBus {
  entries = $state<TimelineEntry[]>([]);
  #seq = 0;

  emit(event: DebugEvent): void {
    this.entries.push({ id: this.#seq++, ts: Date.now(), ...event });
    if (this.entries.length > RING_CAP) {
      this.entries.shift();
    }
  }

  clear(): void {
    this.entries = [];
  }

  get capacity(): number {
    return RING_CAP;
  }
}
