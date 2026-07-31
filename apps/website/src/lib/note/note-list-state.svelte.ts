/* eslint-disable svelte/prefer-svelte-reactivity -- Internal collections are projected into the sole reactive visible array. */

export type NoteListIdentity = {
  siteId: string;
  entityId?: string;
  status: string;
};

export type NotePresence = 'entering' | 'settled' | 'exiting';

export type VisibleNote<T> = {
  note: T;
  presence: NotePresence;
  deleting: boolean;
};

type NoteListStateOptions = {
  isTerminallyDeleted: (siteId: string, noteId: string) => boolean;
  isPendingDeletion?: (noteId: string) => boolean;
};

type ListNote = {
  id: string;
  order: string;
  status: string;
};

const sameIdentity = (left: NoteListIdentity, right: NoteListIdentity): boolean =>
  left.siteId === right.siteId && left.entityId === right.entityId && left.status === right.status;

const sameOrder = (left: readonly string[], right: readonly string[]): boolean =>
  left.length === right.length && left.every((id, index) => id === right[index]);

export class NoteListState<T extends ListNote> {
  readonly #isTerminallyDeleted: NoteListStateOptions['isTerminallyDeleted'];
  readonly #isPendingDeletion: NonNullable<NoteListStateOptions['isPendingDeletion']>;
  #identity: NoteListIdentity | null = null;
  #authoritativeNotesById = new Map<string, T>();
  #authoritativeOrder: string[] = [];
  #enteringIds = new Set<string>();
  #exitingNotesById = new Map<string, T>();
  #restoredExitingNotesById = new Map<string, T>();
  #deletingNotesById = new Map<string, T>();
  #reordering = false;
  #dragPreview: string[] | null = null;
  #desiredOrder: string[] | null = null;
  #visible = $state<VisibleNote<T>[]>([]);

  constructor({ isTerminallyDeleted, isPendingDeletion = () => false }: NoteListStateOptions) {
    this.#isTerminallyDeleted = isTerminallyDeleted;
    this.#isPendingDeletion = isPendingDeletion;
  }

  #clear(): void {
    this.#identity = null;
    this.#authoritativeNotesById.clear();
    this.#authoritativeOrder = [];
    this.#enteringIds.clear();
    this.#exitingNotesById.clear();
    this.#restoredExitingNotesById.clear();
    this.#deletingNotesById.clear();
    this.#reordering = false;
    this.#dragPreview = null;
    this.#desiredOrder = null;
  }

  #isTerminal(noteId: string): boolean {
    return this.#identity !== null && this.#isTerminallyDeleted(this.#identity.siteId, noteId);
  }

  #sortedNotes(notes: readonly T[], identity: NoteListIdentity): T[] {
    return notes
      .filter(
        (note) =>
          note.status === identity.status &&
          !this.#isTerminallyDeleted(identity.siteId, note.id) &&
          (!this.#isPendingDeletion(note.id) || this.#deletingNotesById.has(note.id)),
      )
      .toSorted((left, right) => left.order.localeCompare(right.order));
  }

  #reset(identity: NoteListIdentity, authoritativeNotes: readonly T[]): void {
    this.#clear();
    this.#identity = { ...identity };

    for (const note of this.#sortedNotes(authoritativeNotes, identity)) {
      this.#authoritativeNotesById.set(note.id, note);
      this.#authoritativeOrder.push(note.id);
    }
    this.#rebuildVisible();
  }

  #removeLocalPresence(noteId: string): void {
    this.#enteringIds.delete(noteId);
    this.#restoredExitingNotesById.delete(noteId);
    this.#deletingNotesById.delete(noteId);
  }

  #currentNote(noteId: string): T | undefined {
    return (
      this.#exitingNotesById.get(noteId) ??
      this.#deletingNotesById.get(noteId) ??
      this.#restoredExitingNotesById.get(noteId) ??
      this.#authoritativeNotesById.get(noteId)
    );
  }

  #authoritativeConfirmsDesiredOrder(): boolean {
    const desiredOrder = this.#desiredOrder;
    if (desiredOrder === null) return false;

    const authoritativeIds = new Set(this.#authoritativeOrder);
    const desiredIds = new Set(desiredOrder);
    const desiredCommonOrder = desiredOrder.filter((noteId) => authoritativeIds.has(noteId));
    const authoritativeCommonOrder = this.#authoritativeOrder.filter((noteId) => desiredIds.has(noteId));
    return sameOrder(desiredCommonOrder, authoritativeCommonOrder);
  }

  #rebuildVisible(): void {
    const notesById = new Map(this.#authoritativeNotesById);

    for (const [noteId, note] of this.#restoredExitingNotesById) {
      if (!this.#isTerminal(noteId)) notesById.set(noteId, note);
    }
    for (const [noteId, note] of this.#deletingNotesById) {
      if (!this.#isTerminal(noteId)) notesById.set(noteId, note);
    }
    for (const [noteId, note] of this.#exitingNotesById) {
      notesById.set(noteId, note);
    }

    const fallbackOrder = [...notesById.values()].toSorted((left, right) => left.order.localeCompare(right.order)).map((note) => note.id);
    const preferredOrder = this.#dragPreview ?? this.#desiredOrder ?? fallbackOrder;
    const orderedIds = preferredOrder.filter((noteId) => notesById.has(noteId));
    for (const noteId of fallbackOrder) {
      if (!orderedIds.includes(noteId)) orderedIds.push(noteId);
    }

    this.#visible = orderedIds.map((noteId) => {
      const note = notesById.get(noteId);
      if (!note) throw new Error(`Visible note ${noteId} is missing its snapshot`);
      return {
        note,
        presence: this.#exitingNotesById.has(noteId) ? 'exiting' : this.#enteringIds.has(noteId) ? 'entering' : 'settled',
        deleting: this.#deletingNotesById.has(noteId),
      };
    });
  }

  sync(identity: NoteListIdentity, authoritativeNotes: readonly T[]): void {
    if (this.#identity === null || !sameIdentity(this.#identity, identity)) {
      this.#reset(identity, authoritativeNotes);
      return;
    }

    const previousAuthoritativeNotesById = this.#authoritativeNotesById;
    const nextNotes = this.#sortedNotes(authoritativeNotes, identity);
    const nextAuthoritativeNotesById = new Map(nextNotes.map((note) => [note.id, note]));

    for (const noteId of this.#restoredExitingNotesById.keys()) {
      if (this.#isTerminal(noteId)) this.#removeLocalPresence(noteId);
    }

    for (const note of nextNotes) {
      if (previousAuthoritativeNotesById.has(note.id)) {
        if (this.#deletingNotesById.has(note.id)) this.#deletingNotesById.set(note.id, note);
        continue;
      }

      if (this.#restoredExitingNotesById.delete(note.id) || this.#exitingNotesById.delete(note.id)) {
        this.#enteringIds.delete(note.id);
      } else {
        this.#enteringIds.add(note.id);
      }
    }

    for (const [noteId, previousNote] of previousAuthoritativeNotesById) {
      if (nextAuthoritativeNotesById.has(noteId)) continue;

      if (this.#isTerminal(noteId)) {
        if (!this.#exitingNotesById.has(noteId)) this.#removeLocalPresence(noteId);
      } else if (this.#deletingNotesById.has(noteId)) {
        this.#deletingNotesById.set(noteId, previousNote);
      } else if (!this.#exitingNotesById.has(noteId)) {
        this.#enteringIds.delete(noteId);
        this.#exitingNotesById.set(noteId, previousNote);
      }
    }

    this.#authoritativeNotesById = nextAuthoritativeNotesById;
    this.#authoritativeOrder = nextNotes.map((note) => note.id);

    if (!this.#reordering) {
      this.#dragPreview = null;
      if (this.#authoritativeConfirmsDesiredOrder()) this.#desiredOrder = null;
    }

    this.#rebuildVisible();
  }

  settle(identity: NoteListIdentity, authoritativeNotes: readonly T[]): void {
    this.#reset(identity, authoritativeNotes);
  }

  authoritativeNote(noteId: string): T | undefined {
    return this.#authoritativeNotesById.get(noteId);
  }

  authoritativeOrders(): ReadonlyMap<string, string> {
    return new Map([...this.#authoritativeNotesById].map(([noteId, note]) => [noteId, note.order]));
  }

  desiredOrder(): readonly string[] | null {
    return this.#desiredOrder;
  }

  advanceAuthoritativeOrder(noteId: string, order: string): boolean {
    const note = this.#authoritativeNotesById.get(noteId);
    if (!note) return false;

    const previousOrder = this.#authoritativeOrder;
    this.#authoritativeNotesById.set(noteId, { ...note, order });
    this.#authoritativeOrder = [...this.#authoritativeNotesById.values()]
      .toSorted((left, right) => left.order.localeCompare(right.order))
      .map((item) => item.id);
    if (!this.#reordering && this.#authoritativeConfirmsDesiredOrder()) this.#desiredOrder = null;
    this.#rebuildVisible();
    return !sameOrder(previousOrder, this.#authoritativeOrder);
  }

  visibleNotes(): readonly VisibleNote<T>[] {
    return this.#visible;
  }

  reset(): void {
    this.#clear();
    this.#visible = [];
  }

  markDeleting(noteId: string): boolean {
    if (this.#deletingNotesById.has(noteId) || this.#exitingNotesById.has(noteId) || this.#isTerminal(noteId)) return false;

    const note = this.#currentNote(noteId);
    if (!note) return false;

    this.#deletingNotesById.set(noteId, note);
    this.#rebuildVisible();
    return true;
  }

  clearDeleting(noteId: string): boolean {
    const note = this.#deletingNotesById.get(noteId);
    if (!note) return false;

    this.#deletingNotesById.delete(noteId);
    if (
      !this.#authoritativeNotesById.has(noteId) &&
      !this.#restoredExitingNotesById.has(noteId) &&
      !this.#isTerminal(noteId) &&
      !this.#exitingNotesById.has(noteId)
    ) {
      this.#enteringIds.delete(noteId);
      this.#exitingNotesById.set(noteId, note);
    }
    this.#rebuildVisible();
    return true;
  }

  restoreExiting(noteId: string): boolean {
    if (this.#identity === null || this.#isTerminal(noteId) || this.#restoredExitingNotesById.has(noteId)) return false;

    const note = this.#exitingNotesById.get(noteId);
    if (!note) return false;

    this.#exitingNotesById.delete(noteId);
    this.#enteringIds.delete(noteId);
    this.#restoredExitingNotesById.set(noteId, note);
    this.#rebuildVisible();
    return true;
  }

  resumeExiting(noteId: string): boolean {
    const note = this.#restoredExitingNotesById.get(noteId);
    if (!note) return false;
    this.#restoredExitingNotesById.delete(noteId);
    if (this.#isTerminal(noteId) || this.#authoritativeNotesById.has(noteId)) {
      this.#rebuildVisible();
      return false;
    }

    this.#enteringIds.delete(noteId);
    this.#exitingNotesById.set(noteId, note);
    this.#rebuildVisible();
    return true;
  }

  markDeleted(note: T): boolean {
    if (this.#identity === null || !this.#isTerminal(note.id) || this.#exitingNotesById.has(note.id) || !this.#currentNote(note.id)) {
      return false;
    }

    this.#authoritativeNotesById.delete(note.id);
    this.#authoritativeOrder = this.#authoritativeOrder.filter((noteId) => noteId !== note.id);
    this.#removeLocalPresence(note.id);
    this.#exitingNotesById.set(note.id, note);
    this.#rebuildVisible();
    return true;
  }

  finishEntering(noteId: string): boolean {
    if (!this.#enteringIds.delete(noteId)) return false;
    this.#rebuildVisible();
    return true;
  }

  finishExiting(noteId: string): boolean {
    if (!this.#exitingNotesById.delete(noteId)) return false;
    this.#removeLocalPresence(noteId);
    this.#authoritativeNotesById.delete(noteId);
    this.#authoritativeOrder = this.#authoritativeOrder.filter((id) => id !== noteId);
    this.#rebuildVisible();
    return true;
  }

  beginReorder(noteIds: readonly string[]): boolean {
    const visibleIds = this.#visible.map(({ note }) => note.id);
    if (
      this.#identity === null ||
      !sameOrder(
        noteIds.toSorted((left, right) => left.localeCompare(right)),
        visibleIds.toSorted((left, right) => left.localeCompare(right)),
      )
    ) {
      return false;
    }

    this.#reordering = true;
    this.#dragPreview = [...noteIds];
    this.#rebuildVisible();
    return true;
  }

  setDesiredOrder(noteIds: readonly string[]): boolean {
    const visibleIds = this.#visible.map(({ note }) => note.id);
    if (
      !this.#reordering ||
      !sameOrder(
        noteIds.toSorted((left, right) => left.localeCompare(right)),
        visibleIds.toSorted((left, right) => left.localeCompare(right)),
      )
    ) {
      return false;
    }

    this.#reordering = false;
    this.#dragPreview = null;
    this.#desiredOrder = [...noteIds];
    this.#rebuildVisible();
    return true;
  }

  rollbackOrder(): boolean {
    if (this.#reordering) {
      this.#reordering = false;
      this.#dragPreview = null;
      this.#rebuildVisible();
      return true;
    }
    if (this.#desiredOrder === null) return false;

    this.#desiredOrder = null;
    this.#rebuildVisible();
    return true;
  }

  restoreAuthoritativeOrder(): void {
    if (!this.#reordering && this.#dragPreview === null && this.#desiredOrder === null) return;

    this.#reordering = false;
    this.#dragPreview = null;
    this.#desiredOrder = null;
    this.#rebuildVisible();
  }
}
