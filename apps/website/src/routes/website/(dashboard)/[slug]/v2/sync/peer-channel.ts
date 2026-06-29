export class PeerChannel {
  #channel: BroadcastChannel | null;

  constructor(documentId: string, onMessage: (changeset: Uint8Array) => void) {
    this.#channel = typeof BroadcastChannel === 'undefined' ? null : new BroadcastChannel(`typie:doc:${documentId}`);
    if (this.#channel) {
      this.#channel.addEventListener('message', (e: MessageEvent) => {
        const data = e.data as ArrayBuffer;
        onMessage(new Uint8Array(data));
      });
    }
  }

  post(changeset: Uint8Array): void {
    this.#channel?.postMessage(new Uint8Array(changeset).buffer);
  }

  close(): void {
    this.#channel?.close();
    this.#channel = null;
  }
}
