import { Encoder } from 'cbor-x';
import type { SyncSocketLike } from './connection';
import type { ClientMessage } from './protocol';

const serverEncoder = new Encoder({ useRecords: false });

export class FakeWebSocket implements SyncSocketLike {
  binaryType = 'arraybuffer';
  sent: ClientMessage[] = [];
  closed: { code?: number; reason?: string } | null = null;
  closeCompletes = true;
  onopen: (() => void) | null = null;
  onmessage: ((event: { data: ArrayBuffer }) => void) | null = null;
  onclose: ((event: { code: number }) => void) | null = null;
  onerror: (() => void) | null = null;

  send(data: Uint8Array): void {
    this.sent.push(serverEncoder.decode(data) as ClientMessage);
  }

  close(code?: number, reason?: string): void {
    this.closed = { code, reason };
    if (this.closeCompletes) this.onclose?.({ code: code ?? 1000 });
  }

  serverOpen(): void {
    this.onopen?.();
  }

  serverSend(message: unknown): void {
    const bytes = serverEncoder.encode(message);
    this.onmessage?.({ data: Uint8Array.from(bytes).buffer });
  }

  serverClose(code: number): void {
    this.onclose?.({ code });
  }

  lastOf<T extends ClientMessage['t']>(t: T): (ClientMessage & { t: T }) | undefined {
    return this.sent.findLast((m) => m.t === t) as (ClientMessage & { t: T }) | undefined;
  }
}
