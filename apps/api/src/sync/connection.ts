import { randomUUID } from 'node:crypto';
import { logger } from '@typie/lib';
import { DocumentChannel } from './channel.ts';
import { CLOSE_AUTH_FAILED, CLOSE_BACKPRESSURE, CLOSE_PROTOCOL_ERROR, decodeClientMessage, encodeMessage } from './protocol.ts';
import { handlePull, handlePush } from './requests.ts';
import type { AssetNonceItem, AssetStateEntry, ClientMessage, ServerMessage, SnapshotCursor } from './protocol.ts';
import type { DocumentAccess, SyncDeps, SyncSession } from './types.ts';

export const MAX_BUFFERED_BYTES = 4 * 1024 * 1024;
export const PUSH_BUCKET_CAPACITY = 300;
export const PUSH_BUCKET_REFILL_PER_SECOND = 5;
export const FRAME_WARN_BYTES = 1024 * 1024;
export const ASSET_STATE_MAX_FRAME_BYTES = 1024 * 1024;
export const HELLO_TIMEOUT_MS = 15_000;
export const WRITABLE_TTL_MS = 30_000;

const log = logger.getChild('sync');

// eslint-disable-next-line @typescript-eslint/no-empty-function -- fire-and-forget rejection sink
const swallow = (): void => {};

// entry 배열이 아니라 asset-state envelope 전체를 인코딩한 바이트로 판단한다. 한 entry만으로 상한을
// 넘으면 무한 분할 대신 그 entry만 담은 프레임을 그대로 내보내고 경고만 남긴다. 결과가 0건이어도
// 빈 배열 프레임 하나는 반드시 나가서 클라이언트가 요청을 미결로 남기지 않는다.
export const chunkByEncodedBytes = (
  assets: AssetStateEntry[],
  frame: { documentId: string; requestId: string },
  limitBytes: number = ASSET_STATE_MAX_FRAME_BYTES,
): [chunk: AssetStateEntry[], final: boolean][] => {
  const measure = (chunk: AssetStateEntry[]): number =>
    encodeMessage({ t: 'asset-state', documentId: frame.documentId, requestId: frame.requestId, assets: chunk, final: true }).length;

  const chunks: AssetStateEntry[][] = [];
  let current: AssetStateEntry[] = [];

  for (const entry of assets) {
    if (current.length > 0 && measure([...current, entry]) > limitBytes) {
      chunks.push(current);
      current = [];
    }
    current.push(entry);
    if (current.length === 1) {
      const bytes = measure(current);
      if (bytes > limitBytes) {
        log.warn('Oversized asset-state entry {*}', { id: entry.id, bytes });
        chunks.push(current);
        current = [];
      }
    }
  }

  if (current.length > 0) chunks.push(current);
  if (chunks.length === 0) chunks.push([]);

  return chunks.map((chunk, index) => [chunk, index === chunks.length - 1]);
};

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
  // Server-issued presence lease member — unique per (instance, connection), so a
  // dead connection's teardown can never zrem a live reconnect's lease.
  #connectionId = randomUUID();
  #channels = new Map<string, DocumentChannel>();
  #access = new Map<string, DocumentAccess>();
  #pushTokens = PUSH_BUCKET_CAPACITY;
  #pushRefilledAt: number;
  #writable: boolean | undefined;
  #writableCheckedAt = 0;
  #queue: Promise<void> = Promise.resolve();
  #destroyed = false;
  #helloTimer: ReturnType<typeof setTimeout> | null = null;

  constructor(options: { deps: SyncDeps; socket: SyncSocket; now?: () => number; helloTimeoutMs?: number }) {
    this.#deps = options.deps;
    this.#socket = options.socket;
    this.#now = options.now ?? Date.now;
    this.#pushRefilledAt = this.#now();
    this.#helloTimer = setTimeout(() => {
      this.#helloTimer = null;
      if (!this.#session) this.#close(CLOSE_PROTOCOL_ERROR, 'hello timeout');
    }, options.helloTimeoutMs ?? HELLO_TIMEOUT_MS);
    this.#helloTimer.unref?.();
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
      case 'asset-pull': {
        await this.#handleAssetPull(message);
        return;
      }
      case 'asset-heartbeat': {
        await this.#handleAssetHeartbeat(message);
        return;
      }
      case 'asset-failed': {
        await this.#handleAssetFailed(message);
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
    if (this.#helloTimer) {
      clearTimeout(this.#helloTimer);
      this.#helloTimer = null;
    }
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
    await this.#deps.markPresence(message.documentId, this.#connectionId);
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
    void this.#deps.clearPresence(message.documentId, this.#connectionId).catch(swallow);
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
    if (this.#writable === undefined || this.#now() - this.#writableCheckedAt >= WRITABLE_TTL_MS) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- session set before push
      this.#writable = await this.#deps.checkWritable(this.#session!.userId);
      this.#writableCheckedAt = this.#now();
    }
    if (this.#destroyed) return;
    if (!this.#writable) {
      await this.#send({ t: 'error', scope: 'request', id: message.id, code: 'subscription_required', permanent: true });
      return;
    }
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

  async #handleAssetPull(message: { documentId: string; requestId: string; ids: string[] }): Promise<void> {
    const channel = this.#channels.get(message.documentId);
    if (!channel || channel.stopped) {
      log.debug('asset-pull for unattached document ignored {*}', { documentId: message.documentId });
      return;
    }
    const ids = [...new Set(message.ids)];
    // asset 해석은 문서 동기화의 보조 경로다 — 여기서 던지면 연결이 1011로 끊겨 첨부된 모든 문서가
    // 스냅샷 재다운로드를 하게 되므로, 실패는 삼키고 요청을 미응답으로 남긴다(미attach 케이스와 동일).
    // 클라이언트는 무효화·백오프로 재-pull한다.
    let assets: AssetStateEntry[];
    try {
      assets = await this.#deps.resolveAssetStates(ids);
    } catch (err) {
      log.error('asset-pull resolve failed {*}', { documentId: message.documentId, requestId: message.requestId, error: err });
      return;
    }
    if (this.#destroyed || channel.stopped) return;
    for (const [chunk, final] of chunkByEncodedBytes(assets, message)) {
      await this.#send({ t: 'asset-state', documentId: message.documentId, requestId: message.requestId, assets: chunk, final });
    }
  }

  async #handleAssetHeartbeat(message: { items: AssetNonceItem[] }): Promise<void> {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- session set before asset messages
    await this.#deps.extendAssetLeases(message.items, this.#session!.userId);
  }

  async #handleAssetFailed(message: { items: AssetNonceItem[] }): Promise<void> {
    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion -- session set before asset messages
    const cleared = await this.#deps.clearAssetLeases(message.items, this.#session!.userId);
    for (const [assetId, documentId] of cleared) {
      this.#deps.publishAssetChanged(documentId, [assetId]);
    }
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

  // Called from the WS heartbeat: refresh the lease score for every attached
  // document so an idle-but-connected session keeps deferring sweeps.
  refreshPresence(): void {
    for (const [documentId, channel] of this.#channels) {
      if (!channel.stopped) void this.#deps.markPresence(documentId, this.#connectionId).catch(swallow);
    }
  }

  handleMessage(data: Uint8Array): Promise<void> {
    const run = this.#queue.then(() => this.#process(data));
    // eslint-disable-next-line @typescript-eslint/no-empty-function -- swallow rejection; caller awaits run
    this.#queue = run.catch(() => {});
    return run;
  }

  destroy(): void {
    this.#destroyed = true;
    if (this.#helloTimer) {
      clearTimeout(this.#helloTimer);
      this.#helloTimer = null;
    }
    for (const [documentId, channel] of this.#channels) {
      channel.stop();
      void this.#deps.clearPresence(documentId, this.#connectionId).catch(swallow);
    }
    this.#channels.clear();
  }
}
