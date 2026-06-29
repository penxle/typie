import type { DeltaRecord, DeltaStore } from './store';

export const enc = (...ids: number[]) => new Uint8Array(ids);
export const dec = (p: Uint8Array) => [...p].toSorted((a, b) => a - b);

export class FakeEditor {
  known: Set<number>;
  constructor(initial: number[]) {
    this.known = new Set(initial);
  }
  currentHeads() {
    return this.known.size > 0 ? enc(Math.max(...this.known)) : enc();
  }
  missingChangesetsFor(heads: Uint8Array) {
    const hs = dec(heads).filter((id) => this.known.has(id));
    const eff = hs.length > 0 ? Math.max(...hs) : 0;
    return enc(...[...this.known].filter((id) => id > eff).toSorted((a, b) => a - b));
  }
  splitChangesets(p: Uint8Array) {
    return dec(p).map((n) => ({ id: String(n), bytes: enc(n) }));
  }
  partitionRemoteChangesets(p: Uint8Array) {
    const ready = dec(p).filter((id) => !this.known.has(id));
    return { ready: enc(...ready), blocked: enc() };
  }
  receiveRemoteChangeset(p: Uint8Array) {
    for (const id of dec(p)) this.known.add(id);
  }
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- intentional no-op in fake
  flush() {}
}

export class FakeStore implements DeltaStore {
  records: DeltaRecord[] = [];
  async load(documentId: string) {
    return this.records.filter((r) => r.documentId === documentId).toSorted((a, b) => a.createdAt - b.createdAt);
  }
  async put(r: DeltaRecord) {
    this.records = [...this.records.filter((x) => x.id !== r.id), r];
  }
  async deleteMany(documentId: string, ids: string[]) {
    this.records = this.records.filter((r) => !(r.documentId === documentId && ids.includes(r.id)));
  }
  // eslint-disable-next-line @typescript-eslint/no-empty-function -- intentional no-op in fake
  destroy() {}
}
