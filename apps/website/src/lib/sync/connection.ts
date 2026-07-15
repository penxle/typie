import { CLOSE_AUTH_FAILED, CLOSE_PROTOCOL_ERROR, decodeServerMessage, encodeClientMessage, SyncRequestError } from './protocol';
import type { ClientMessage, ServerMessage, SnapshotCursor } from './protocol';

const RECONNECT_BASE_MS = 1000;
const RECONNECT_CAP_MS = 30_000;
const PING_INTERVAL_MS = 30_000;
const PONG_TIMEOUT_MS = 10_000;
const PROBE_TIMEOUT_MS = 5000;
const HELLO_TIMEOUT_MS = 10_000;
const AUTH_FAILED_MAX_STREAK = 3;

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
  pingIntervalMs?: number;
  pongTimeoutMs?: number;
  probeTimeoutMs?: number;
  helloTimeoutMs?: number;
};

type PendingRequest = {
  resolve: (message: ServerMessage) => void;
  reject: (error: Error) => void;
};

type ChannelCursor = { sinceSeq?: string; snapshotCursor?: SnapshotCursor };

export class SyncConnection {
  private readonly opts: ConnectionOpts;
  private readonly clientId = crypto.randomUUID();
  private readonly pingIntervalMs: number;
  private readonly pongTimeoutMs: number;
  private readonly probeTimeoutMs: number;
  private readonly helloTimeoutMs: number;
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
  private pingInFlight = false;
  private probing = false;
  private helloTimer: ReturnType<typeof setTimeout> | null = null;
  private livenessWait: { promise: Promise<void>; resolve: () => void } | null = null;
  private terminal = false;
  private terminalError: SyncRequestError | null = null;
  private authFailedStreak = 0;

  constructor(opts: ConnectionOpts) {
    this.opts = opts;
    this.pingIntervalMs = opts.pingIntervalMs ?? PING_INTERVAL_MS;
    this.pongTimeoutMs = opts.pongTimeoutMs ?? PONG_TIMEOUT_MS;
    this.probeTimeoutMs = opts.probeTimeoutMs ?? PROBE_TIMEOUT_MS;
    this.helloTimeoutMs = opts.helloTimeoutMs ?? HELLO_TIMEOUT_MS;
  }

  private request(message: ClientMessage & { id: string }): Promise<ServerMessage> {
    return new Promise((resolve, reject) => {
      if (this.terminalError) {
        reject(this.terminalError);
        return;
      }
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
    if (this.disposed || this.terminal || this.connecting || this.readySocket || this.reconnectTimer) return;
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
      this.helloTimer = setTimeout(() => {
        this.helloTimer = null;
        if (this.socket === socket && this.readySocket !== socket) this.terminateSocket(socket);
      }, this.helloTimeoutMs);
    };

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onmessage = (event) => {
      const message = decodeServerMessage(new Uint8Array(event.data));
      if (!message) return;
      this.handleMessage(message, socket);
    };

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onerror = () => {
      this.terminateSocket(socket);
    };

    // eslint-disable-next-line unicorn/prefer-add-event-listener -- SyncSocketLike is a structural subset without addEventListener
    socket.onclose = (event) => {
      this.teardown(socket, event.code);
    };
  }

  private teardown(socket: SyncSocketLike, closeCode: number | null): void {
    if (this.socket !== socket) return;
    this.socket = null;
    this.readySocket = null;
    this.connecting = false;
    if (this.helloTimer) {
      clearTimeout(this.helloTimer);
      this.helloTimer = null;
    }
    this.stopPing();
    this.livenessWait?.resolve();
    this.livenessWait = null;
    if (this.classifyClose(closeCode)) {
      this.enterTerminal(closeCode);
      return;
    }
    this.failPending();
    this.outbox = [];
    if (!this.disposed) this.scheduleReconnect();
  }

  private classifyClose(closeCode: number | null): boolean {
    if (closeCode === CLOSE_PROTOCOL_ERROR) return true;
    if (closeCode === CLOSE_AUTH_FAILED) {
      this.authFailedStreak += 1;
      return this.authFailedStreak >= AUTH_FAILED_MAX_STREAK;
    }
    this.authFailedStreak = 0;
    return false;
  }

  private enterTerminal(closeCode: number | null): void {
    if (this.terminal) return;
    this.terminal = true;
    const code = closeCode === CLOSE_PROTOCOL_ERROR ? 'connection_permanent_protocol_error' : 'connection_permanent_auth_failed';
    const error = new SyncRequestError(code, true);
    this.terminalError = error;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
    const entries = [...this.pending.values()];
    this.pending.clear();
    this.outbox = [];
    for (const entry of entries) entry.reject(error);
    const channels = [...this.channelHandlers.entries()];
    for (const [documentId, handlers] of channels) {
      const message: ServerMessage = { t: 'error', scope: 'document', documentId, code, permanent: true };
      const snapshot = [...handlers];
      for (const handler of snapshot) handler(message);
    }
  }

  private terminateSocket(socket: SyncSocketLike): void {
    this.teardown(socket, null);
    socket.close(1000);
  }

  private handleMessage(message: ServerMessage, socket: SyncSocketLike): void {
    if (socket !== this.socket) return;
    this.livenessWait?.resolve();
    this.livenessWait = null;
    if (message.t === 'hello-ack') {
      if (this.helloTimer) {
        clearTimeout(this.helloTimer);
        this.helloTimer = null;
      }
      this.readySocket = socket;
      this.connecting = false;
      this.attempts = 0;
      this.startPing(socket);
      this.flushOutbox(socket);
      for (const callback of this.reconnectedCallbacks) callback();
      return;
    }
    if (message.t === 'pong') {
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

  private startPing(socket: SyncSocketLike): void {
    this.stopPing();
    this.pingTimer = setInterval(() => {
      if (this.pingInFlight) return;
      this.pingInFlight = true;
      void this.awaitLiveness(this.pongTimeoutMs).then((alive) => {
        this.pingInFlight = false;
        if (!alive && this.socket === socket) this.terminateSocket(socket);
      });
    }, this.pingIntervalMs);
  }

  private stopPing(): void {
    if (this.pingTimer) clearInterval(this.pingTimer);
    this.pingTimer = null;
    this.pingInFlight = false;
  }

  private awaitLiveness(timeoutMs: number): Promise<boolean> {
    if (!this.livenessWait) {
      let resolve!: () => void;
      const promise = new Promise<void>((r) => (resolve = r));
      this.livenessWait = { promise, resolve };
    }
    const wait = this.livenessWait;
    this.readySocket?.send(encodeClientMessage({ t: 'ping' }));
    return new Promise((resolve) => {
      let settled = false;
      const timer = setTimeout(() => {
        if (settled) return;
        settled = true;
        if (this.livenessWait === wait) this.livenessWait = null;
        resolve(false);
      }, timeoutMs);
      void wait.promise.then(() => {
        if (settled) return;
        settled = true;
        clearTimeout(timer);
        resolve(true);
      });
    });
  }

  async ensureLive(): Promise<void> {
    if (this.disposed) return;
    const socket = this.readySocket;
    if (!socket) return;
    const alive = await this.awaitLiveness(this.probeTimeoutMs);
    if (!alive && this.socket === socket) this.terminateSocket(socket);
  }

  onForeground(): void {
    if (this.disposed || this.terminal) return;
    if (!this.socket) {
      if (this.reconnectTimer) {
        clearTimeout(this.reconnectTimer);
        this.reconnectTimer = null;
        this.attempts = 0;
        this.ensureConnected();
      }
      return;
    }
    if (!this.readySocket || this.probing) return;
    this.probing = true;
    void this.ensureLive().finally(() => {
      this.probing = false;
    });
  }

  resetTerminal(): void {
    this.terminal = false;
    this.terminalError = null;
    this.authFailedStreak = 0;
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
    if (this.terminal && !this.channelHandlers.has(documentId)) this.resetTerminal();
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
    if (this.helloTimer) {
      clearTimeout(this.helloTimer);
      this.helloTimer = null;
    }
    this.stopPing();
    this.livenessWait?.resolve();
    this.livenessWait = null;
    this.socket?.close(1000);
    this.failPending();
  }
}
