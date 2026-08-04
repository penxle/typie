import type { ChannelSubscriber } from '$lib/sync/channel';

export type RemoteChangesetEvent = Parameters<ChannelSubscriber['onChangesets']>[0];

type ChangesetReceiver = {
  receiveRemoteChangeset(payload: Uint8Array): Promise<number>;
};

// TODO: Move channel subscription, polling/refetch, and resync lifecycle from
// DocumentEditor into this class. KMP RemoteChangesetPipeline already owns those
// responsibilities. This narrower Web version currently owns one Editor's remote
// event application and ordered seq/heads updates; admissions stay immediate so
// events delivered together can still share a tick.
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
