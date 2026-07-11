import { logger } from '@typie/lib';
import { DocumentChannel } from './channel.ts';
import { CLOSE_AUTH_FAILED, CLOSE_BACKPRESSURE, CLOSE_PROTOCOL_ERROR, decodeClientMessage, encodeMessage } from './protocol.ts';
import { handlePull, handlePush } from './requests.ts';
import type { ClientMessage, ServerMessage, SnapshotCursor } from './protocol.ts';
import type { DocumentAccess, SyncDeps, SyncSession } from './types.ts';

export const MAX_BUFFERED_BYTES = 4 * 1024 * 1024;
export const PUSH_BUCKET_CAPACITY = 300;
export const PUSH_BUCKET_REFILL_PER_SECOND = 5;
export const FRAME_WARN_BYTES = 1024 * 1024;

const log = logger.getChild('sync');

export type SyncSocket = {
  send: (data: Uint8Array) => Promise<void>;
  close: (code: number, reason?: string) => void;
  bufferedAmount: () => number;
};

export class SyncConnection {
  #deps: SyncDeps;
  #socket: SyncSocket;
  #now: () => number;
  #session: SyncSession | null = null;
  #clientId = '';
  #channels = new Map<string, DocumentChannel>();
  #access = new Map<string, DocumentAccess>();
  #pushTokens = PUSH_BUCKET_CAPACITY;
  #pushRefilledAt: number;
  #queue: Promise<void> = Promise.resolve();
  #destroyed = false;

  constructor(options: { deps: SyncDeps; socket: SyncSocket; now?: () => number }) {
    this.#deps = options.deps;
    this.#socket = options.socket;
    this.#now = options.now ?? Date.now;
    this.#pushRefilledAt = this.#now();
  }

  async #process(data: Uint8Array): Promise<void> {
    if (this.#destroyed) return;
    const result = decodeClientMessage(data);
    if (!result.ok) {
      if (result.reason === 'malformed') this.#close(CLOSE_PROTOCOL_ERROR, 'malformed message');
      else if (this.#session) {
        log.debug('Unknown message type ignored {*}', { type: result.type });
      } else {
        this.#close(CLOSE_PROTOCOL_ERROR, 'hello required');
      }
      return;
    }
    const message = result.message;
    if (!this.#session) {
      if (message.t !== 'hello') {
        this.#close(CLOSE_PROTOCOL_ERROR, 'hello required');
        return;
      }
      await this.#handleHello(message);
      return;
    }
    switch (message.t) {
      case 'hello': {
        this.#close(CLOSE_PROTOCOL_ERROR, 'duplicate hello');
        return;
      }
      case 'ping': {
        await this.#send({ t: 'pong' });
        return;
      }
      case 'attach': {
        await this.#handleAttach(message);
        return;
      }
      case 'detach': {
        this.#handleDetach(message);
        return;
      }
      case 'push': {
        await this.#handlePush(message);
        return;
      }
      case 'pull': {
        await this.#handlePull(message);
        return;
      }
    }
  }

  #close(code: number, reason: string): void {
    this.destroy();
    this.#socket.close(code, reason);
  }

  async #handleHello(message: ClientMessage & { t: 'hello' }): Promise<void> {
    const session = await this.#deps.consumeTicket(message.ticket);
    if (this.#destroyed) return;
    if (!session) {
      this.#close(CLOSE_AUTH_FAILED, 'invalid ticket');
      return;
    }
    this.#session = session;
    this.#clientId = message.clientId;
    await this.#send({ t: 'hello-ack', capabilities: [] });
  }

  async #checkAccess(documentId: string): Promise<DocumentAccess> {
    const cached = this.#access.get(documentId);
    if (cached) return cached;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- session set before access checks
    const access = await this.#deps.checkDocumentAccess(this.#session!.userId, documentId);
    this.#access.set(documentId, access);
    return access;
  }

  async #handleAttach(message: { documentId: string; sinceSeq?: string; snapshotCursor?: SnapshotCursor }): Promise<void> {
    if (message.sinceSeq !== undefined && message.snapshotCursor !== undefined) {
      this.#close(CLOSE_PROTOCOL_ERROR, 'conflicting cursors');
      return;
    }
    const existing = this.#channels.get(message.documentId);
    if (existing && !existing.stopped) {
      this.#close(CLOSE_PROTOCOL_ERROR, 'duplicate attach');
      return;
    }
    const access = await this.#checkAccess(message.documentId);
    if (this.#destroyed) return;
    if (access !== 'ok') {
      await this.#send({
        t: 'error',
        scope: 'document',
        documentId: message.documentId,
        code: access === 'not_v2' ? 'document_not_v2' : 'forbidden',
        permanent: true,
      });
      return;
    }
    const channel = new DocumentChannel({
      deps: this.#deps,
      send: (m) => this.#send(m),
      documentId: message.documentId,
      clientId: this.#clientId,
      onOverload: () => this.#close(CLOSE_BACKPRESSURE, 'live buffer overflow'),
    });
    this.#channels.set(message.documentId, channel);
    void channel.start({ sinceSeq: message.sinceSeq, snapshotCursor: message.snapshotCursor }).catch(async () => {
      channel.stop();
      if (this.#channels.get(message.documentId) !== channel) return;
      try {
        await this.#send({ t: 'error', scope: 'document', documentId: message.documentId, code: 'internal', permanent: false });
      } catch {
        // 소켓이 이미 죽었으면 전달 불가 — 연결 종료 경로가 정리한다
      }
    });
  }

  #handleDetach(message: { documentId: string }): void {
    this.#channels.get(message.documentId)?.stop();
    this.#channels.delete(message.documentId);
  }

  #takePushToken(): boolean {
    const now = this.#now();
    const elapsed = (now - this.#pushRefilledAt) / 1000;
    this.#pushTokens = Math.min(PUSH_BUCKET_CAPACITY, this.#pushTokens + elapsed * PUSH_BUCKET_REFILL_PER_SECOND);
    this.#pushRefilledAt = now;
    if (this.#pushTokens < 1) return false;
    this.#pushTokens -= 1;
    return true;
  }

  async #handlePush(message: { id: string; documentId: string; changesets: Uint8Array }): Promise<void> {
    if (!this.#takePushToken()) {
      await this.#send({ t: 'error', scope: 'request', id: message.id, code: 'rate_limited', permanent: false });
      return;
    }
    const access = await this.#checkAccess(message.documentId);
    if (access !== 'ok') {
      await this.#send({
        t: 'error',
        scope: 'request',
        id: message.id,
        code: access === 'not_v2' ? 'document_not_v2' : 'forbidden',
        permanent: true,
      });
      return;
    }
    if (this.#destroyed) return;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- session set before push
    await handlePush({ deps: this.#deps, session: this.#session!, clientId: this.#clientId }, message, (m) => this.#send(m));
  }

  async #handlePull(message: { id: string; documentId: string; sinceSeq?: string }): Promise<void> {
    const access = await this.#checkAccess(message.documentId);
    if (access !== 'ok') {
      await this.#send({
        t: 'error',
        scope: 'request',
        id: message.id,
        code: access === 'not_v2' ? 'document_not_v2' : 'forbidden',
        permanent: true,
      });
      return;
    }
    if (this.#destroyed) return;
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- session set before pull
    await handlePull({ deps: this.#deps, session: this.#session!, clientId: this.#clientId }, message, (m) => this.#send(m));
  }

  async #send(message: ServerMessage): Promise<void> {
    if (this.#destroyed) return;
    if (this.#socket.bufferedAmount() > MAX_BUFFERED_BYTES) {
      this.#close(CLOSE_BACKPRESSURE, 'backpressure overflow');
      return;
    }
    const data = encodeMessage(message);
    if (data.length > FRAME_WARN_BYTES) {
      log.warn('Oversized frame {*}', { type: message.t, bytes: data.length });
    }
    await this.#socket.send(data);
    if (this.#socket.bufferedAmount() > MAX_BUFFERED_BYTES) {
      this.#close(CLOSE_BACKPRESSURE, 'backpressure overflow');
    }
  }

  get bootstrapBypassKeyHash(): string | undefined {
    return this.#session?.bootstrapBypassKeyHash;
  }

  handleMessage(data: Uint8Array): Promise<void> {
    const run = this.#queue.then(() => this.#process(data));
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- swallow rejection; caller awaits run
    this.#queue = run.catch(() => {});
    return run;
  }

  destroy(): void {
    this.#destroyed = true;
    for (const channel of this.#channels.values()) channel.stop();
    this.#channels.clear();
  }
}
