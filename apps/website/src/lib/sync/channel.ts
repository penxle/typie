import { compareStreamSeq } from './protocol';
import type { SyncConnection } from './connection';
import type { ServerMessage, SnapshotCursor } from './protocol';

export type ChannelSubscriber = {
  onSnapshot: (graph: Uint8Array, meta: { seq: string; heads: Uint8Array; durableHeads: Uint8Array }) => void;
  onChangesets: (event: { seq: string; bundles: Uint8Array[]; heads: Uint8Array; durableHeads: Uint8Array }) => void;
  onReload: () => void;
  onPermanentError: (code: string) => void;
};

type SubscriberState = { subscriber: ChannelSubscriber; loading: boolean; lastSeq: string | null };

type ChannelState = {
  documentId: string;
  subscribers: Set<SubscriberState>;
  chunks: Uint8Array[];
  accumulated: number;
  cursor: SnapshotCursor | null;
  probe: SnapshotCursor | null;
  sinceSeq: string | null;
  snapshotDone: boolean;
  awaitingAck: boolean;
  retryAttempts: number;
  retryTimer: ReturnType<typeof setTimeout> | null;
  ackTimer: ReturnType<typeof setTimeout> | null;
  unregister: () => void;
};

const concat = (chunks: Uint8Array[], total: number): Uint8Array => {
  const out = new Uint8Array(total);
  let offset = 0;
  for (const part of chunks) {
    out.set(part, offset);
    offset += part.length;
  }
  return out;
};

export class DocumentChannels {
  private readonly connection: SyncConnection;
  private readonly channels = new Map<string, ChannelState>();
  private readonly retryBaseMs: number;
  private readonly attachAckTimeoutMs: number;

  constructor(connection: SyncConnection, retryBaseMs = 1000, attachAckTimeoutMs = 10_000) {
    this.connection = connection;
    this.retryBaseMs = retryBaseMs;
    this.attachAckTimeoutMs = attachAckTimeoutMs;
    this.connection.onReconnected(() => {
      for (const channel of this.channels.values()) this.reattach(channel);
    });
  }

  private resetSnapshot(channel: ChannelState): void {
    channel.chunks = [];
    channel.accumulated = 0;
    channel.cursor = null;
    channel.probe = null;
  }

  private reattach(channel: ChannelState): void {
    if (channel.retryTimer) {
      clearTimeout(channel.retryTimer);
      channel.retryTimer = null;
    }
    this.armAckWatchdog(channel);
    if (channel.snapshotDone) {
      this.connection.sendAttach(channel.documentId, { sinceSeq: channel.sinceSeq ?? '' });
      return;
    }
    if (!channel.snapshotDone && channel.cursor) {
      channel.probe = channel.cursor;
      this.connection.sendAttach(channel.documentId, { snapshotCursor: channel.cursor });
      return;
    }
    this.connection.sendAttach(channel.documentId, {});
  }

  private restartChannel(channel: ChannelState): void {
    this.resetSnapshot(channel);
    channel.snapshotDone = false;
    channel.awaitingAck = true;
    this.armAckWatchdog(channel);
    if (this.connection.ready) {
      this.connection.sendDetach(channel.documentId);
      this.connection.sendAttach(channel.documentId, {});
    }
  }

  private clearAckWatchdog(channel: ChannelState): void {
    if (channel.ackTimer) clearTimeout(channel.ackTimer);
    channel.ackTimer = null;
  }

  private armAckWatchdog(channel: ChannelState): void {
    this.clearAckWatchdog(channel);
    channel.ackTimer = setTimeout(() => {
      channel.ackTimer = null;
      void this.handleAckTimeout(channel);
    }, this.attachAckTimeoutMs);
  }

  private async handleAckTimeout(channel: ChannelState): Promise<void> {
    if (this.channels.get(channel.documentId) !== channel) return;
    await this.connection.ensureLive();
    if (this.channels.get(channel.documentId) !== channel) return;
    if (this.connection.ready) this.connection.sendDetach(channel.documentId);
    this.reattach(channel);
  }

  private scheduleRetry(channel: ChannelState): void {
    if (channel.retryTimer) return;
    channel.retryAttempts += 1;
    const delay = Math.min(this.retryBaseMs * 2 ** (channel.retryAttempts - 1), 30_000);
    channel.retryTimer = setTimeout(() => {
      channel.retryTimer = null;
      this.reattach(channel);
    }, delay);
  }

  private append(channel: ChannelState, message: ServerMessage & { t: 'snapshot-chunk' }): void {
    channel.chunks.push(message.bytes);
    channel.accumulated += message.bytes.length;
    channel.cursor = { rowId: message.rowId, seq: message.seq, offset: message.offset + message.bytes.length };
  }

  private handle(channel: ChannelState, message: ServerMessage): void {
    switch (message.t) {
      case 'attach-ack': {
        channel.awaitingAck = false;
        this.clearAckWatchdog(channel);
        return;
      }
      case 'snapshot-chunk': {
        if (channel.awaitingAck) return;
        if (channel.probe) {
          const probe = channel.probe;
          channel.probe = null;
          if (message.rowId === probe.rowId && message.offset === probe.offset) {
            this.append(channel, message);
            return;
          }
          this.resetSnapshot(channel);
        }
        if (channel.cursor === null) {
          if (message.offset !== 0) {
            this.restartChannel(channel);
            return;
          }
        } else {
          const continues =
            (message.rowId === channel.cursor.rowId && message.offset === channel.cursor.offset) ||
            (message.rowId !== channel.cursor.rowId && message.offset === 0);
          if (!continues) {
            this.restartChannel(channel);
            return;
          }
        }
        this.append(channel, message);
        return;
      }
      case 'snapshot-end': {
        if (channel.awaitingAck) return;
        channel.retryAttempts = 0;
        const graph = concat(channel.chunks, channel.accumulated);
        this.resetSnapshot(channel);
        channel.snapshotDone = true;
        if (message.seq !== '' && (channel.sinceSeq === null || compareStreamSeq(message.seq, channel.sinceSeq) > 0)) {
          channel.sinceSeq = message.seq;
        }
        for (const state of channel.subscribers) {
          if (!state.loading) continue;
          state.loading = false;
          state.lastSeq = message.seq === '' ? null : message.seq;
          state.subscriber.onSnapshot(graph, { seq: message.seq, heads: message.heads, durableHeads: message.durableHeads });
        }
        return;
      }
      case 'changesets': {
        if (channel.awaitingAck) return;
        if (message.seq !== '' && (channel.sinceSeq === null || compareStreamSeq(message.seq, channel.sinceSeq) > 0)) {
          channel.sinceSeq = message.seq;
        }
        for (const state of channel.subscribers) {
          if (state.loading) continue;
          if (message.seq !== '' && state.lastSeq !== null && compareStreamSeq(message.seq, state.lastSeq) <= 0) continue;
          if (message.seq !== '') state.lastSeq = message.seq;
          state.subscriber.onChangesets({
            seq: message.seq,
            bundles: message.bundles,
            heads: message.heads,
            durableHeads: message.durableHeads,
          });
        }
        return;
      }
      case 'reload': {
        this.clearAckWatchdog(channel);
        for (const state of channel.subscribers) state.subscriber.onReload();
        return;
      }
      case 'error': {
        if (message.scope !== 'document') return;
        this.clearAckWatchdog(channel);
        if (message.permanent) {
          if (channel.retryTimer) clearTimeout(channel.retryTimer);
          for (const state of channel.subscribers) state.subscriber.onPermanentError(message.code);
          channel.unregister();
          this.channels.delete(channel.documentId);
          return;
        }
        this.resetSnapshot(channel);
        this.scheduleRetry(channel);
        return;
      }
      default: {
        return;
      }
    }
  }

  subscribe(documentId: string, subscriber: ChannelSubscriber): () => void {
    const state: SubscriberState = { subscriber, loading: true, lastSeq: null };
    const channel = this.channels.get(documentId);

    if (channel) {
      channel.subscribers.add(state);
      this.restartChannel(channel);
    } else {
      const created: ChannelState = {
        documentId,
        subscribers: new Set([state]),
        chunks: [],
        accumulated: 0,
        cursor: null,
        probe: null,
        sinceSeq: null,
        snapshotDone: false,
        awaitingAck: false,
        retryAttempts: 0,
        retryTimer: null,
        ackTimer: null,
        // eslint-disable-next-line @typescript-eslint/no-empty-function -- overwritten immediately below; registerChannel's handler must close over `created`
        unregister: () => {},
      };
      created.unregister = this.connection.registerChannel(documentId, (message) => this.handle(created, message));
      this.channels.set(documentId, created);
      this.armAckWatchdog(created);
      this.connection.sendAttach(documentId, {});
    }

    return () => {
      const current = this.channels.get(documentId);
      if (!current) return;
      current.subscribers.delete(state);
      if (current.subscribers.size === 0) {
        if (current.retryTimer) clearTimeout(current.retryTimer);
        this.clearAckWatchdog(current);
        current.unregister();
        this.channels.delete(documentId);
        this.connection.sendDetach(documentId);
      }
    };
  }
}

export const loadDocumentSnapshot = (channels: DocumentChannels, documentId: string): Promise<Uint8Array> =>
  new Promise((resolve, reject) => {
    const off = channels.subscribe(documentId, {
      onSnapshot: (graph) => {
        off();
        resolve(graph);
      },
      // eslint-disable-next-line @typescript-eslint/no-empty-function -- loadDocumentSnapshot only resolves on the terminal snapshot event
      onChangesets: () => {},
      onReload: () => {
        off();
        reject(new Error('reloaded'));
      },
      onPermanentError: (code) => {
        off();
        reject(new Error(code));
      },
    });
  });
