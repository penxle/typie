import type { ChannelSubscriber } from '$lib/sync/channel';

export type RemoteChangesetEvent = Parameters<ChannelSubscriber['onChangesets']>[0];

type ChangesetReceiver = {
  receiveRemoteChangeset(payload: Uint8Array): Promise<number>;
};

// Apply remote events and advance their metadata in order. Admissions stay
// immediate so events delivered together can share a tick; DocumentEditingSession
// owns the surrounding polling, peer connection and editor lifetime.
export class RemoteChangesetPipeline {
  readonly #editor: ChangesetReceiver;
  readonly #onApplied: (event: RemoteChangesetEvent) => void;
  #metadataTail: Promise<void> = Promise.resolve();

  constructor(editor: ChangesetReceiver, onApplied: (event: RemoteChangesetEvent) => void) {
    this.#editor = editor;
    this.#onApplied = onApplied;
  }

  apply(event: RemoteChangesetEvent): Promise<void> {
    const applied = event.bundles.filter((payload) => payload.length > 0).map((payload) => this.#editor.receiveRemoteChangeset(payload));
    const settled = Promise.all(applied)
      .then(() => ({ type: 'applied' }) as const)
      .catch((err: unknown) => ({ type: 'failed', error: err }) as const);
    const completed = this.#metadataTail.then(async () => {
      const result = await settled;
      if (result.type === 'failed') throw result.error;
      this.#onApplied(event);
    });
    this.#metadataTail = completed;
    return completed;
  }
}
