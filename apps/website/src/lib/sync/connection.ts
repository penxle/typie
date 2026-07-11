import { CLOSE_PROTOCOL_ERROR, decodeServerMessage, encodeClientMessage, SyncRequestError } from './protocol';
import type { ClientMessage, ServerMessage, SnapshotCursor } from './protocol';

const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 30_000;
const PING_INTERVAL_MS = 30_000;
const PING_MAX_MISSES = 2;

export type SyncSocketLike = {
  binaryType: string;
  send: (data: Uint8Array) => void;
  close: (code?: number, reason?: string) => void;
  onopen: (() => void) | null;
  onmessage: ((event: { data: ArrayBuffer }) => void) | null;
  onclose: ((event: { code: number }) => void) | null;
  onerror: (() => void) | null;
};

export type ConnectionOpts = {
  createSocket: () => SyncSocketLike;
  fetchTicket: () => Promise<string>;
};

type PendingRequest = {
  resolve: (message: ServerMessage) => void;
  reject: (error: Error) => void;
};

type ChannelCursor = { sinceSeq?: string; snapshotCursor?: SnapshotCursor };

export class SyncConnection {
  private readonly opts: ConnectionOpts;
  private readonly clientId = crypto.randomUUID();
  private socket: SyncSocketLike | null = null;
  private readySocket: SyncSocketLike | null = null;
  private connecting = false;
  private disposed = false;
  private attempts = 0;
  private requestSeq = 0;
  private readonly pending = new Map<string, PendingRequest>();
  private outbox: (ClientMessage & { id: string })[] = [];
  private readonly channelHandlers = new Map<string, Set<(message: ServerMessage) => void>>();
  private readonly reconnectedCallbacks = new Set<() => void>();
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private pingTimer: ReturnType<typeof setInterval> | null = null;
  private missedPongs = 0;

  constructor(opts: ConnectionOpts) {
    this.opts = opts;
  }

  private request(message: ClientMessage & { id: string }): Promise<ServerMessage> {
    return new Promise((resolve, reject) => {
      this.pending.set(message.id, { resolve, reject });
      this.outbox.push(message);
      if (this.readySocket) this.flushOutbox(this.readySocket);
      else this.ensureConnected();
    });
  }

  private flushOutbox(socket: SyncSocketLike): void {
    const queued = [...this.outbox];
    this.outbox.length = 0;
    for (const item of queued) socket.send(encodeClientMessage(item));
  }

  private ensureConnected(): void {
    if (this.disposed || this.connecting || this.readySocket || this.reconnectTimer) return;
    this.connecting = true;
    void this.connect();
  }

  private async connect(): Promise<void> {
    let ticket: string;
    try {
      ticket = await this.opts.fetchTicket();
    } catch {
      this.connecting = false;
      this.scheduleReconnect();
      return;
    }
    if (this.disposed) {
      this.connecting = false;
      return;
    }

    const socket = this.opts.createSocket();
    this.socket = socket;
    socket.binaryType = 'arraybuffer';

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onopen = () => {
      socket.send(encodeClientMessage({ t: 'hello', ticket, clientId: this.clientId, capabilities: [] }));
    };

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onmessage = (event) => {
      const message = decodeServerMessage(new Uint8Array(event.data));
      if (!message) return;
      this.handleMessage(message, socket);
    };

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onerror = () => {
      socket.close(CLOSE_PROTOCOL_ERROR);
    };

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onclose = () => {
      if (this.socket !== socket) return;
      this.socket = null;
      this.readySocket = null;
      this.connecting = false;
      this.stopPing();
      this.failPending();
      this.outbox = [];
      if (!this.disposed) this.scheduleReconnect();
    };
  }

  private handleMessage(message: ServerMessage, socket: SyncSocketLike): void {
    if (socket !== this.socket) return;
    if (message.t === 'hello-ack') {
      this.readySocket = socket;
      this.connecting = false;
      this.attempts = 0;
      this.startPing();
      this.flushOutbox(socket);
      for (const callback of this.reconnectedCallbacks) callback();
      return;
    }
    if (message.t === 'pong') {
      this.missedPongs = 0;
      return;
    }
    if (message.t === 'push-ack' || message.t === 'pull-ack' || (message.t === 'error' && message.scope === 'request')) {
      const id = message.id;
      if (!id) return;
      const entry = this.pending.get(id);
      if (!entry) return;
      this.pending.delete(id);
      if (message.t === 'error') entry.reject(new SyncRequestError(message.code, message.permanent));
      else entry.resolve(message);
      return;
    }
    if ('documentId' in message && message.documentId) {
      const handlers = this.channelHandlers.get(message.documentId);
      if (!handlers) return;
      for (const handler of handlers) handler(message);
    }
  }

  private failPending(): void {
    const entries = [...this.pending.values()];
    this.pending.clear();
    for (const entry of entries) entry.reject(new SyncRequestError('connection_lost', false));
  }

  private scheduleReconnect(): void {
    if (this.disposed || this.reconnectTimer) return;
    if (this.outbox.length === 0 && this.channelHandlers.size === 0 && this.pending.size === 0) return;
    this.attempts += 1;
    const delay = Math.min(RECONNECT_BASE_MS * 2 ** (this.attempts - 1), RECONNECT_CAP_MS);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connecting = true;
      void this.connect();
    }, delay);
  }

  private startPing(): void {
    this.missedPongs = 0;
    this.pingTimer = setInterval(() => {
      if (this.missedPongs >= PING_MAX_MISSES) {
        this.socket?.close(CLOSE_PROTOCOL_ERROR, 'ping timeout');
        return;
      }
      this.missedPongs += 1;
      this.readySocket?.send(encodeClientMessage({ t: 'ping' }));
    }, PING_INTERVAL_MS);
  }

  private stopPing(): void {
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.pingTimer = null;
    this.missedPongs = 0;
  }

  get ready(): boolean {
    return this.readySocket !== null;
  }

  push(documentId: string, changesets: Uint8Array): Promise<{ heads: Uint8Array; durableHeads: Uint8Array }> {
    const id = `r${++this.requestSeq}`;
    return this.request({ t: 'push', id, documentId, changesets }).then((message) => {
      if (message.t !== 'push-ack') throw new SyncRequestError('unexpected_response', false);
      return { heads: message.heads, durableHeads: message.durableHeads };
    });
  }

  pull(
    documentId: string,
    sinceSeq: string | null,
  ): Promise<{ changesets: Uint8Array[]; seq: string; heads: Uint8Array; durableHeads: Uint8Array; needsReload: boolean }> {
    const id = `r${++this.requestSeq}`;
    const message: ClientMessage = sinceSeq === null ? { t: 'pull', id, documentId } : { t: 'pull', id, documentId, sinceSeq };
    return this.request(message).then((response) => {
      if (response.t !== 'pull-ack') throw new SyncRequestError('unexpected_response', false);
      return {
        changesets: response.changesets,
        seq: response.seq,
        heads: response.heads,
        durableHeads: response.durableHeads,
        needsReload: response.needsReload,
      };
    });
  }

  registerChannel(documentId: string, handler: (message: ServerMessage) => void): () => void {
    const set = this.channelHandlers.get(documentId) ?? new Set();
    set.add(handler);
    this.channelHandlers.set(documentId, set);
    return () => {
      set.delete(handler);
      if (set.size === 0) this.channelHandlers.delete(documentId);
    };
  }

  sendAttach(documentId: string, cursor: ChannelCursor = {}): void {
    if (this.readySocket) {
      this.readySocket.send(
        encodeClientMessage({ t: 'attach', documentId, sinceSeq: cursor.sinceSeq, snapshotCursor: cursor.snapshotCursor }),
      );
      return;
    }
    this.ensureConnected();
  }

  sendDetach(documentId: string): void {
    this.readySocket?.send(encodeClientMessage({ t: 'detach', documentId }));
  }

  onReconnected(callback: () => void): () => void {
    this.reconnectedCallbacks.add(callback);
    return () => this.reconnectedCallbacks.delete(callback);
  }

  dispose(): void {
    this.disposed = true;
    if (this.reconnectTimer) clearTimeout(this.reconnectTimer);
    this.stopPing();
    this.socket?.close(1000);
    this.failPending();
  }
}
