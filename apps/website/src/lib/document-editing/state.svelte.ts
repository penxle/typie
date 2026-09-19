import { untrack } from 'svelte';
import { DocumentPreparation } from './session';
import type { NavigationPreparation, NavigationRequest } from '$lib/navigation';
import type { DocumentEditingSession } from './session';

export type DocumentSaveReason = NavigationRequest['reason'] | 'save' | 'sync';

class DocumentOperation {
  #attempt = 0;
  #progressTimer: ReturnType<typeof setTimeout> | undefined;
  #dialogShown = false;
  #finished = $state(false);
  readonly reason: DocumentSaveReason;
  readonly preparation: DocumentPreparation;
  readonly result = Promise.withResolvers<'protected' | 'discard' | 'cancel'>();
  waiting = $state(true);
  displayedSessions = $state.raw<readonly DocumentEditingSession[]>([]);
  showProgress = $state(false);

  constructor(reason: DocumentSaveReason, preparation: DocumentPreparation) {
    this.reason = reason;
    this.preparation = preparation;
  }

  get dialogShown(): boolean {
    return this.#dialogShown;
  }

  get phase(): 'saving' | 'blocked' | 'protected' | 'finished' {
    if (this.#finished) return 'finished';
    if (this.preparation.isProtected()) return 'protected';
    return this.waiting ? 'saving' : 'blocked';
  }

  get unprotectedSessions(): DocumentEditingSession[] {
    return this.preparation.sessions.filter((session) => !this.preparation.isSessionProtected(session));
  }

  get canCancel(): boolean {
    return this.reason === 'leave' || this.reason === 'logout' || this.reason === 'save' || this.reason === 'sync';
  }

  markDialogShown(): void {
    if (this.displayedSessions.length > 0) this.#dialogShown = true;
  }

  async retry(): Promise<void> {
    if (this.phase === 'finished') return;
    const attempt = ++this.#attempt;
    this.waiting = true;
    this.showProgress = false;
    clearTimeout(this.#progressTimer);
    this.#progressTimer = setTimeout(() => {
      this.showProgress = true;
    }, 350);
    const failures = await this.preparation.checkpoint();
    if (attempt !== this.#attempt) return;
    clearTimeout(this.#progressTimer);
    this.displayedSessions = [...new Set([...this.displayedSessions, ...failures])];
    this.waiting = false;
  }

  finish(proceed: 'protected' | 'discard' | 'cancel'): void {
    if (this.phase === 'finished') return;
    this.#finished = true;
    this.#attempt++;
    clearTimeout(this.#progressTimer);
    this.result.resolve(proceed);
  }
}

class DocumentEditingState {
  #listeners = new Set<() => void>();
  #departing = new Map<DocumentEditingSession, number>();
  #nativePreparations = $state(0);
  sessions = $state.raw<readonly DocumentEditingSession[]>([]);
  operations = $state.raw<readonly DocumentOperation[]>([]);
  nativeProgress = $state(false);
  #changed(): void {
    untrack(() => {
      for (const operation of this.operations) {
        if (operation.preparation.sessions.some((session) => session.disposed)) operation.finish('cancel');
        // Only an already displayed dialog needs a completion screen. A fast,
        // locally safe departure must not wait for unrelated server-save details.
        else if (operation.phase === 'protected' && (this.nativePreparations > 0 || !operation.dialogShown)) operation.finish('protected');
      }
      for (const listener of this.#listeners) listener();
    });
  }

  async #resolve(operation: DocumentOperation, startCheckpoint: boolean): Promise<'protected' | 'discard' | 'cancel'> {
    const { preparation } = operation;
    this.operations = [...this.operations, operation];
    if (!startCheckpoint && !preparation.isProtected()) {
      // Opening save details does not start another checkpoint. Existing writes
      // continue; only the dialog's Retry action requests a new attempt.
      operation.waiting = false;
      operation.displayedSessions = operation.unprotectedSessions;
    } else {
      void operation.retry().then(() => this.#changed());
    }
    this.#changed();
    try {
      return await operation.result.promise;
    } finally {
      this.operations = this.operations.filter((entry) => entry !== operation);
      this.#changed();
    }
  }

  get completed(): boolean {
    const active = this.operations.filter((operation) => operation.phase !== 'finished');
    return active.length > 0 && active.every((operation) => operation.phase === 'protected');
  }

  get nativePreparations(): number {
    return this.#nativePreparations;
  }

  set nativePreparations(value: number) {
    this.#nativePreparations = value;
    this.#changed();
  }

  completeRecovery(): void {
    const active = this.operations.filter((operation) => operation.phase !== 'finished');
    if (active.length === 0 || active.some((operation) => !operation.preparation.isProtected())) {
      this.#changed();
      return;
    }
    for (const operation of active) {
      operation.finish('protected');
    }
  }

  register(session: DocumentEditingSession): () => void {
    const { localEdits } = session.editor;
    const isInputAllowed = localEdits.isInputAllowed;
    localEdits.isInputAllowed = () => this.nativePreparations === 0 && isInputAllowed();
    this.sessions = [...this.sessions, session];
    const unsubscribe = session.onChange(() => {
      if (session.disposed) {
        unsubscribe();
        localEdits.isInputAllowed = isInputAllowed;
        this.sessions = this.sessions.filter((entry) => entry !== session);
      }
      this.#changed();
    });
    this.#changed();
    return () => session.dispose();
  }

  hasUnprotectedChanges(): boolean {
    return this.sessions.some((session) => !this.#departing.has(session) && !session.isProtected());
  }

  onChange(listener: () => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  markDialogShown(): void {
    for (const operation of this.operations) operation.markDialogShown();
  }

  authorizeDeparture(sessions: readonly DocumentEditingSession[]): () => void {
    for (const session of sessions) this.#departing.set(session, (this.#departing.get(session) ?? 0) + 1);
    this.#changed();
    return () => {
      for (const session of sessions) {
        const remaining = (this.#departing.get(session) ?? 0) - 1;
        if (remaining > 0) this.#departing.set(session, remaining);
        else this.#departing.delete(session);
      }
      this.#changed();
    };
  }

  async prepareDeparture(
    getSessions: () => readonly DocumentEditingSession[],
    reason: NavigationRequest['reason'],
  ): Promise<NavigationPreparation | false> {
    let sessions = getSessions();
    if (sessions.some((session) => session.disposed || !this.sessions.includes(session))) return false;
    const isCurrent = () => {
      const current = getSessions();
      return (
        current.length === sessions.length &&
        sessions.every((session) => !session.disposed && this.sessions.includes(session) && current.includes(session))
      );
    };
    let preparation = new DocumentPreparation(sessions.filter((session) => !this.#departing.has(session)));
    let retained = false;
    try {
      while (true) {
        const proceed =
          preparation.sessions.length > 0 ? await this.#resolve(new DocumentOperation(reason, preparation), true) : 'protected';
        if (proceed === 'cancel') return false;
        const current = getSessions();
        if (
          current.some((session) => session.disposed || !this.sessions.includes(session)) ||
          sessions.some((session) => !current.includes(session))
        ) {
          return false;
        }
        if (current.length !== sessions.length) {
          // A pane can finish loading while departure is pending. Stop the expanded
          // selection before releasing the old one, and do not reuse its discard decision.
          const next = new DocumentPreparation(current.filter((session) => !this.#departing.has(session)));
          preparation.release();
          preparation = next;
          sessions = current;
          continue;
        }
        if (proceed === 'discard' || preparation.isProtected()) break;
      }
      const revoke = this.authorizeDeparture(preparation.sessions);
      retained = true;
      return {
        isCurrent,
        release: () => {
          if (!retained) return;
          retained = false;
          revoke();
          preparation.release();
        },
      };
    } finally {
      if (!retained) preparation.release();
    }
  }

  async showSaveStatus(sessions: readonly DocumentEditingSession[], requireServer = false): Promise<boolean> {
    if (sessions.some((session) => session.disposed || !this.sessions.includes(session))) return false;
    const preparation = new DocumentPreparation(sessions, requireServer);
    try {
      const proceed = await this.#resolve(new DocumentOperation(requireServer ? 'sync' : 'save', preparation), false);
      return proceed === 'protected';
    } finally {
      preparation.release();
    }
  }

  retry(): void {
    for (const operation of this.operations) {
      if (operation.phase === 'blocked') void operation.retry().then(() => this.#changed());
    }
  }

  cancel(): void {
    for (const operation of this.operations) {
      if (operation.canCancel) operation.finish('cancel');
    }
    this.#changed();
  }

  discard(): void {
    for (const operation of this.operations) operation.finish('discard');
    this.#changed();
  }

  async capture(): Promise<void> {
    await Promise.allSettled(this.sessions.map((session) => session.capture()));
  }
}

export const documentEditing = new DocumentEditingState();
