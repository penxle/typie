import { createSubscriber } from 'svelte/reactivity';
import { GapBuffer } from '../../routes/website/(dashboard)/[slug]/v2/sync/gap-buffer';
import { PeerChannel } from '../../routes/website/(dashboard)/[slug]/v2/sync/peer-channel';
import { Pusher } from '../../routes/website/(dashboard)/[slug]/v2/sync/pusher.svelte';
import { RemoteChangesetPipeline } from '../../routes/website/(dashboard)/[slug]/v2/sync/remote-changeset-pipeline';
import type { DocumentSaveState } from '@typie/lib/document-save';
import type { Editor } from '$lib/editor-ffi/editor.svelte';
import type { SyncConnection } from '$lib/sync/connection';
import type { RemoteChangesetEvent } from '../../routes/website/(dashboard)/[slug]/v2/sync/remote-changeset-pipeline';
import type { IndexeddbDeltaStore } from '../../routes/website/(dashboard)/[slug]/v2/sync/store';

export type DocumentSaveStatus = 'idle' | 'pending' | 'failed' | 'sync-failed' | 'synced';

export class DocumentEditingSession {
  static create({
    documentId,
    paneId,
    title,
    entity,
    editor,
    store,
    snapshot,
    connection,
    onReload,
    onError,
  }: {
    documentId: string;
    paneId: string;
    title: () => string;
    entity: DocumentEditingSession['entity'];
    editor: Editor;
    store: IndexeddbDeltaStore;
    snapshot: { heads: Uint8Array; durableHeads: Uint8Array; seq: string };
    connection: Pick<SyncConnection, 'push' | 'pull'>;
    onReload: () => void;
    onError: (error: unknown) => void;
  }): DocumentEditingSession {
    let seq = snapshot.seq;
    const refetch = async () => {
      const result = await connection.pull(documentId, seq || null);
      if (session.disposed) return;
      if (result.needsReload) {
        onReload();
        return;
      }
      await Promise.all(result.changesets.filter((bytes) => bytes.length > 0).map((bytes) => editor.receiveRemoteChangeset(bytes)));
      if (session.disposed) return;
      if (result.seq) seq = result.seq;
      pusher.setConfirmedHeads(result.heads);
      pusher.setDurableHeads(result.durableHeads);
    };
    const refetchInBackground = () => void refetch().catch(onError);
    const gap = new GapBuffer({
      partition: (payload) => editor.partitionRemoteChangesets(payload),
      apply: (ready) => void editor.receiveRemoteChangeset(ready).catch(onError),
      onStuck: refetchInBackground,
    });
    const peer = new PeerChannel(documentId, (changesets) => gap.ingest(changesets));
    const pusher = new Pusher({
      editor,
      documentId,
      initialServerHeads: snapshot.heads,
      initialDurableHeads: snapshot.durableHeads,
      store,
      // eslint-disable-next-line unicorn/no-return-array-push -- SyncConnection.push returns a server acknowledgement
      pushFn: (changesets) => connection.push(documentId, changesets),
      broadcast: (changesets) => peer.post(changesets),
    });
    const session = new DocumentEditingSession(documentId, paneId, title, editor, pusher, entity);
    session.#pipeline = new RemoteChangesetPipeline(editor, (event) => {
      if (session.disposed) return;
      if (event.seq) seq = event.seq;
      if (event.bundles.length === 0 && event.seq) return;
      pusher.setConfirmedHeads(event.heads);
      pusher.setDurableHeads(event.durableHeads);
    });
    const poll = setInterval(refetchInBackground, 10_000);
    session.#disposeResources = () => {
      clearInterval(poll);
      peer.close();
      // The Svelte subtree must detach before releasing the core it renders.
      queueMicrotask(() => {
        editor.destroy();
        store.destroy();
      });
    };
    return session;
  }

  readonly #listeners = new Set<() => void>();
  // The same change stream drives synchronous departure checks and Svelte views.
  readonly #subscribe = createSubscriber((update) => this.onChange(update));
  readonly #unsubscribe: (() => void)[];
  #stops = 0;
  #finalizeError: unknown;
  #disposed = false;
  #checkpointFailed = false;
  #pipeline: RemoteChangesetPipeline | undefined;
  #disposeResources: (() => void) | undefined;
  readonly id = crypto.randomUUID();
  readonly documentId: string;
  readonly paneId: string;
  readonly title: () => string;
  readonly entity?: () => { icon: string; iconColor: string } | undefined;
  readonly editor: Pick<Editor, 'terminal' | 'documentRevision' | 'finalizeInput' | 'settlePendingEdits'> & {
    localEdits: Pick<Editor['localEdits'], 'pending' | 'stop' | 'onChange'>;
  };
  readonly pusher: Pick<
    Pusher,
    | 'isProtected'
    | 'isSynced'
    | 'checkpoint'
    | 'pushNow'
    | 'captureNow'
    | 'schedule'
    | 'onProtectionChange'
    | 'captureFailures'
    | 'pushFailed'
    | 'stop'
  >;

  constructor(
    documentId: string,
    paneId: string,
    title: () => string,
    editor: DocumentEditingSession['editor'],
    pusher: DocumentEditingSession['pusher'],
    entity?: DocumentEditingSession['entity'],
  ) {
    this.documentId = documentId;
    this.paneId = paneId;
    this.title = title;
    this.entity = entity;
    this.editor = editor;
    this.pusher = pusher;
    let observedRevision = editor.documentRevision;
    this.#unsubscribe = [
      editor.localEdits.onChange(() => {
        if (editor.documentRevision !== observedRevision) {
          observedRevision = editor.documentRevision;
          pusher.schedule();
        }
        this.#notify();
      }),
      pusher.onProtectionChange(() => {
        if (this.isProtected()) this.#checkpointFailed = false;
        this.#notify();
      }),
    ];
  }

  #notify(): void {
    for (const listener of this.#listeners) listener();
  }

  applyRemoteChangesets(event: RemoteChangesetEvent): Promise<void> {
    if (this.#disposed) return Promise.resolve();
    if (!this.#pipeline) throw new Error('Document sync is not initialized');
    return this.#pipeline.apply(event);
  }

  get disposed(): boolean {
    return this.#disposed;
  }

  get saveFailed(): boolean {
    return this.#checkpointFailed && !this.isProtected();
  }

  get saveStatus(): DocumentSaveStatus | null {
    this.#subscribe();
    if (this.#disposed || this.editor.terminal) return null;
    try {
      if (this.saveFailed || (this.pusher.captureFailures > 0 && this.pusher.pushFailed && !this.pusher.isProtected())) return 'failed';
      // Departure protection includes active composition; storage feedback must not
      // mistake that input for slow storage or hide an already confirmed push error.
      if (this.pusher.isSynced()) return this.isSynced() ? 'synced' : 'idle';
      return this.pusher.pushFailed ? 'sync-failed' : 'pending';
    } catch {
      return 'failed';
    }
  }

  get saveFailedOnBoth(): boolean {
    return this.saveStatus === 'failed' && this.pusher.captureFailures > 0 && this.pusher.pushFailed;
  }

  markSaveFailure(): void {
    this.#checkpointFailed = true;
    this.#notify();
  }

  isProtected(): boolean {
    this.#subscribe();
    if (this.#disposed || this.editor.terminal || this.editor.localEdits.pending || this.#finalizeError !== undefined) return false;
    try {
      return this.pusher.isProtected();
    } catch {
      return false;
    }
  }

  isSynced(): boolean {
    this.#subscribe();
    if (this.#disposed || this.editor.terminal || this.editor.localEdits.pending || this.#finalizeError !== undefined) return false;
    try {
      return this.pusher.isSynced();
    } catch {
      return false;
    }
  }

  onChange(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  beginStop(): () => void {
    if (this.#disposed) throw new Error('Document editing session is disposed');
    if (this.#stops === 0) {
      this.#finalizeError = undefined;
      try {
        this.editor.finalizeInput();
      } catch (err) {
        this.#finalizeError = err;
      }
    }
    const releaseEditor = this.editor.localEdits.stop();
    this.#stops++;
    let released = false;
    return () => {
      if (released) return;
      released = true;
      this.#stops--;
      releaseEditor();
      if (this.#stops === 0) {
        this.#finalizeError = undefined;
        this.#notify();
      }
    };
  }

  async checkpoint(requireServer = false): Promise<void> {
    if (this.#disposed) throw new Error('Document editing session is disposed');
    if (this.#finalizeError !== undefined) throw this.#finalizeError;
    this.editor.settlePendingEdits();
    if (requireServer) await this.pusher.pushNow();
    else await this.pusher.checkpoint();
    if (!(requireServer ? this.isSynced() : this.isProtected())) throw new Error('Document changes are not protected');
  }

  async capture(): Promise<void> {
    if (this.#disposed || this.editor.terminal) return;
    this.editor.settlePendingEdits();
    await this.pusher.captureNow();
  }

  dispose(): void {
    if (this.#disposed) return;
    this.#disposed = true;
    for (const unsubscribe of this.#unsubscribe) unsubscribe();
    this.pusher.stop();
    this.#disposeResources?.();
    this.#notify();
    this.#listeners.clear();
  }
}

export class DocumentPreparation {
  readonly #release: (() => void)[] = [];
  readonly #pendingCheckpoints = new Map<DocumentEditingSession, Promise<void>>();
  #released = false;
  readonly sessions: readonly DocumentEditingSession[];
  readonly requireServer: boolean;

  constructor(sessions: readonly DocumentEditingSession[], requireServer = false) {
    this.sessions = sessions;
    this.requireServer = requireServer;
    try {
      for (const session of sessions) this.#release.push(session.beginStop());
    } catch (err) {
      this.release();
      throw err;
    }
  }

  #checkpointSession(session: DocumentEditingSession): Promise<void> {
    const pending = this.#pendingCheckpoints.get(session);
    if (pending) return pending;
    const checkpoint = session
      .checkpoint(this.requireServer)
      .finally(() => this.#pendingCheckpoints.delete(session))
      .catch((err: unknown) => {
        // A slow checkpoint is not a failure. Only the actual save rejection
        // changes the header, including failures after we stopped waiting.
        session.markSaveFailure();
        throw err;
      });
    this.#pendingCheckpoints.set(session, checkpoint);
    return checkpoint;
  }

  async #waitForProtection(session: DocumentEditingSession): Promise<boolean> {
    if (this.isSessionProtected(session)) return true;
    const result = Promise.withResolvers<boolean>();
    const recheck = () => {
      if (session.disposed) result.resolve(false);
      else if (this.isSessionProtected(session)) result.resolve(true);
    };
    const unsubscribe = session.onChange(recheck);
    // Stop waiting after three seconds; do not cancel the in-flight save.
    const timer = setTimeout(() => result.resolve(false), 3000);
    try {
      void this.#checkpointSession(session)
        .then(recheck)
        .catch(() => result.resolve(false));
      recheck();
      return await result.promise;
    } finally {
      clearTimeout(timer);
      unsubscribe();
    }
  }

  isProtected(): boolean {
    return !this.#released && this.sessions.every((session) => this.isSessionProtected(session));
  }

  isSessionProtected(session: DocumentEditingSession): boolean {
    return this.requireServer ? session.isSynced() : session.isProtected();
  }

  getSessionStatus(session: DocumentEditingSession): DocumentSaveState {
    if (session.isSynced()) return 'synced';
    const status = session.saveStatus;
    if (status === null) return 'unknown';
    if (this.isSessionProtected(session)) return status === 'sync-failed' ? 'sync-failed' : 'protected';
    if (status === 'pending' || this.#pendingCheckpoints.has(session)) return 'pending';
    if (status === 'sync-failed' && !this.requireServer) return 'pending';
    return session.isProtected() ? 'sync-failed' : 'failed';
  }

  async checkpoint(): Promise<DocumentEditingSession[]> {
    const remaining = await Promise.all(
      this.sessions.map(async (session) => {
        if (await this.#waitForProtection(session)) return null;
        return session;
      }),
    );
    return remaining.filter((session) => session !== null);
  }

  release(): void {
    if (this.#released) return;
    this.#released = true;
    for (const release of this.#release) release();
  }
}
