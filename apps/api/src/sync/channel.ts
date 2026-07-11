import { compareStreamSeq } from './protocol.ts';
import type { ServerMessage, SnapshotCursor } from './protocol.ts';
import type { BundleRow, ChangesetEvent, ChangesetSubscription, StreamEntry, SyncDeps } from './types.ts';

export const SNAPSHOT_CHUNK_BYTES = 256 * 1024;
export const TAIL_BATCH_ENTRIES = 64;
export const TAIL_BATCH_BYTES = 256 * 1024;
export const BUNDLE_READ_LIMIT = 16;
export const LIVE_BUFFER_MAX_BYTES = 4 * 1024 * 1024;
export const LIVE_BUFFER_EVENT_OVERHEAD = 256;

export type ChannelCursor = { sinceSeq?: string; snapshotCursor?: SnapshotCursor };

type SendFn = (message: ServerMessage) => Promise<void>;

export class DocumentChannel {
  #deps: SyncDeps;
  #sendFn: SendFn;
  #documentId: string;
  #clientId: string;
  #phase: 'loading' | 'live' | 'stopped' = 'loading';
  #buffer: ChangesetEvent[] = [];
  #bufferBytes = 0;
  #subscription: ChangesetSubscription | null = null;
  #onOverload: () => void;

  constructor(options: { deps: SyncDeps; send: SendFn; documentId: string; clientId: string; onOverload?: () => void }) {
    this.#deps = options.deps;
    this.#sendFn = options.send;
    this.#documentId = options.documentId;
    this.#clientId = options.clientId;
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- optional callback default no-op
    this.#onOverload = options.onOverload ?? (() => {});
  }

  async #send(message: ServerMessage): Promise<void> {
    if (this.#phase === 'stopped') return;
    await this.#sendFn(message);
  }

  async #runPump(subscription: ChangesetSubscription): Promise<void> {
    for await (const event of subscription) {
      if (this.#phase === 'stopped') return;
      if (this.#phase === 'live') {
        await this.#emitEvent(event);
      } else {
        if (!this.#accepts(event.target)) continue;
        this.#buffer.push(event);
        this.#bufferBytes += LIVE_BUFFER_EVENT_OVERHEAD + event.changesets.reduce((n, c) => n + c.length, 0);
        if (this.#bufferBytes > LIVE_BUFFER_MAX_BYTES) {
          this.stop();
          this.#onOverload();
          return;
        }
      }
    }
  }

  #accepts(target: string): boolean {
    if (target === '*') return true;
    if (target.startsWith('!')) return target.slice(1) !== this.#clientId;
    return target === this.#clientId;
  }

  async #emitEvent(event: ChangesetEvent): Promise<void> {
    if (!this.#accepts(event.target)) return;
    await this.#send({
      t: 'changesets',
      documentId: this.#documentId,
      seq: event.seq,
      bundles: event.changesets.map((c) => Uint8Array.fromBase64(c)),
      heads: Uint8Array.fromBase64(event.heads),
      durableHeads: Uint8Array.fromBase64(event.durableHeads),
    });
  }

  async #readHeads(): Promise<{ heads: Uint8Array; durableHeads: Uint8Array }> {
    const [live, durable] = await Promise.all([this.#deps.getLiveHeads(this.#documentId), this.#deps.getDurableHeads(this.#documentId)]);
    const durableHeads = durable ?? new Uint8Array();
    return { heads: live ?? durableHeads, durableHeads };
  }

  async #startFromSnapshot(cursor?: SnapshotCursor): Promise<void> {
    const collectedSeq = await this.#deps.getCollectedSeq(this.#documentId);
    let afterSeq = 0;
    if (cursor) {
      const row = await this.#deps.readBundleRow(this.#documentId, cursor.rowId);
      if (row && row.seq === cursor.seq && cursor.offset <= row.payload.length) {
        if (cursor.offset === row.payload.length) {
          await this.#send({
            t: 'snapshot-chunk',
            documentId: this.#documentId,
            rowId: row.id,
            seq: row.seq,
            offset: cursor.offset,
            bytes: new Uint8Array(),
          });
        } else {
          await this.#sendRowChunks(row, cursor.offset);
        }
        afterSeq = row.seq;
      }
    }
    for (;;) {
      if (this.#phase === 'stopped') return;
      const rows = await this.#deps.readBundlesAfter(this.#documentId, afterSeq, BUNDLE_READ_LIMIT);
      for (const row of rows) await this.#sendRowChunks(row, 0);
      if (rows.length < BUNDLE_READ_LIMIT) break;
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
      afterSeq = rows.at(-1)!.seq;
    }
    if (this.stopped) return;
    const { heads, durableHeads } = await this.#readHeads();
    await this.#send({ t: 'snapshot-end', documentId: this.#documentId, seq: collectedSeq ?? '', heads, durableHeads });
    await this.#sendTail(collectedSeq);
  }

  async #sendRowChunks(row: BundleRow, startOffset: number): Promise<void> {
    for (let offset = startOffset; offset < row.payload.length; offset += SNAPSHOT_CHUNK_BYTES) {
      if (this.#phase === 'stopped') return;
      await this.#send({
        t: 'snapshot-chunk',
        documentId: this.#documentId,
        rowId: row.id,
        seq: row.seq,
        offset,
        bytes: row.payload.subarray(offset, offset + SNAPSHOT_CHUNK_BYTES),
      });
    }
  }

  async #startFromStream(sinceSeq: string): Promise<void> {
    const cursor = sinceSeq === '' ? null : sinceSeq;
    const reload = async (): Promise<void> => {
      await this.#send({ t: 'reload', documentId: this.#documentId });
      this.stop();
    };
    if (cursor === null) {
      if ((await this.#deps.getCollectedSeq(this.#documentId)) !== null) {
        const tip = await this.#deps.streamTip(this.#documentId);
        if (tip === null || (await this.#deps.hasStreamBeenTrimmed(this.#documentId))) {
          await reload();
          return;
        }
      }
    } else if ((await this.#deps.streamTip(this.#documentId)) === null) {
      await reload();
      return;
    }
    await this.#sendTail(cursor);
  }

  async #sendTail(sinceSeq: string | null): Promise<void> {
    const reload = async (): Promise<void> => {
      await this.#send({ t: 'reload', documentId: this.#documentId });
      this.stop();
    };
    const tip = await this.#deps.streamTip(this.#documentId);
    if (sinceSeq !== null && tip !== null && compareStreamSeq(sinceSeq, tip) > 0) {
      await reload();
      return;
    }
    let batch: StreamEntry[] = [];
    let batchBytes = 0;
    const flush = async (): Promise<void> => {
      if (batch.length === 0) return;
      const { heads, durableHeads } = await this.#readHeads();
      await this.#send({
        t: 'changesets',
        documentId: this.#documentId,
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
        seq: batch.at(-1)!.seq,
        bundles: batch.map((e) => e.changeset),
        heads,
        durableHeads,
      });
      batch = [];
      batchBytes = 0;
    };
    let cursor = sinceSeq;
    let done = tip === null;
    while (!done) {
      const page = await this.#deps.readStreamBatch(this.#documentId, cursor, TAIL_BATCH_ENTRIES);
      if (cursor !== null && (await this.#deps.isStreamTruncated(this.#documentId, cursor))) {
        await reload();
        return;
      }
      if (page.length === 0) break;
      for (const entry of page) {
        if (this.#phase === 'stopped') return;
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- tip non-null while looping
        if (compareStreamSeq(entry.seq, tip!) > 0) {
          done = true;
          break;
        }
        if (batch.length >= TAIL_BATCH_ENTRIES || (batch.length > 0 && batchBytes + entry.changeset.length > TAIL_BATCH_BYTES)) {
          await flush();
        }
        batch.push(entry);
        batchBytes += entry.changeset.length;
      }
      if (page.length < TAIL_BATCH_ENTRIES) done = true;
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
      cursor = page.at(-1)!.seq;
    }
    await flush();
    await this.#drainAndGoLive();
  }

  async #drainAndGoLive(): Promise<void> {
    while (this.#buffer.length > 0) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- non-empty guarded
      const event = this.#buffer.shift()!;
      this.#bufferBytes -= LIVE_BUFFER_EVENT_OVERHEAD + event.changesets.reduce((n, c) => n + c.length, 0);
      await this.#emitEvent(event);
    }
    if (this.#phase !== 'stopped') this.#phase = 'live';
  }

  get stopped(): boolean {
    return this.#phase === 'stopped';
  }

  async start(cursor: ChannelCursor = {}): Promise<void> {
    await this.#send({ t: 'attach-ack', documentId: this.#documentId });
    if (this.#phase === 'stopped') return;
    const subscription = this.#deps.subscribeChangesets(this.#documentId);
    this.#subscription = subscription;
    void this.#runPump(subscription).catch(() => this.stop());
    if (cursor.sinceSeq === undefined) {
      await this.#startFromSnapshot(cursor.snapshotCursor);
    } else {
      await this.#startFromStream(cursor.sinceSeq);
    }
  }

  stop(): void {
    if (this.#phase === 'stopped') return;
    this.#phase = 'stopped';
    this.#subscription?.return();
  }
}
