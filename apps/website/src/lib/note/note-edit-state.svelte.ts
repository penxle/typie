import { Toast } from '@typie/ui/notification';
import { getContext, setContext, untrack } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';

type NoteFieldSnapshot = {
  content: string;
  color: string;
};

type NoteSaveOutcome =
  { kind: 'saved'; snapshot: NoteFieldSnapshot } | { kind: 'subscription_gated' } | { kind: 'not_found' } | { kind: 'failed' };

type NoteSaveDisplay = 'none' | 'saving' | 'failed';

type NoteField = keyof NoteFieldSnapshot;

type NoteFieldSave = {
  field: NoteField;
  value: string;
};

type NoteEditStateOptions = {
  initialSnapshot: NoteFieldSnapshot;
  save: (request: NoteFieldSave) => Promise<NoteSaveOutcome>;
};

type NoteEditsOptions = {
  isTerminallyDeleted: (siteId: string, noteId: string) => boolean;
  save: (request: NoteFieldSave & { siteId: string; noteId: string }) => Promise<NoteSaveOutcome>;
};

type PendingSave = {
  revision: number;
  value: string;
  onSaved?: () => void;
};

type FieldState = {
  baseline: string;
  revision: number;
  blockedRevision: number | null;
  dirty: boolean;
  failed: boolean;
  onSaved?: () => void;
  debounceTimer: ReturnType<typeof setTimeout> | null;
  queued: PendingSave | null;
};

const DEBOUNCE_MS: Record<NoteField, number> = {
  content: 300,
  color: 180,
};

export class NoteEditState {
  readonly #save: NoteEditStateOptions['save'];
  readonly #contentState: FieldState;
  readonly #colorState: FieldState;
  // Each mutation writes a full note snapshot, so content and color share one request lane.
  #activeSave: (PendingSave & { field: NoteField }) | null = null;
  #queuedFields: NoteField[] = [];
  #displayTimer: ReturnType<typeof setTimeout> | null = null;
  #displayIntervalActive = false;
  #displayDelayElapsed = false;
  #disposed = false;

  content = $state('');
  color = $state('');
  saveDisplay = $state<NoteSaveDisplay>('none');

  constructor({ initialSnapshot, save }: NoteEditStateOptions) {
    this.#save = save;
    this.#contentState = this.#createFieldState(initialSnapshot.content);
    this.#colorState = this.#createFieldState(initialSnapshot.color);
    this.content = initialSnapshot.content;
    this.color = initialSnapshot.color;
  }

  #createFieldState(baseline: string): FieldState {
    return {
      baseline,
      revision: 0,
      blockedRevision: null,
      dirty: false,
      failed: false,
      onSaved: undefined,
      debounceTimer: null,
      queued: null,
    };
  }

  #fieldState(field: NoteField): FieldState {
    return field === 'content' ? this.#contentState : this.#colorState;
  }

  #draft(field: NoteField): string {
    return field === 'content' ? this.content : this.color;
  }

  #setDraft(field: NoteField, value: string): void {
    if (field === 'content') this.content = value;
    else this.color = value;
  }

  #hasFailed(): boolean {
    return this.#contentState.failed || this.#colorState.failed;
  }

  #setFailed(field: NoteField, value: boolean): void {
    const wasFailed = this.#hasFailed();
    this.#fieldState(field).failed = value;

    if (!wasFailed && this.#hasFailed()) {
      Toast.error('노트를 저장하지 못했어요.');
    }
    this.#updateSaveDisplay();
  }

  #clearDebounce(fieldState: FieldState): void {
    if (fieldState.debounceTimer === null) return;
    clearTimeout(fieldState.debounceTimer);
    fieldState.debounceTimer = null;
  }

  #sameSave(left: PendingSave, right: PendingSave): boolean {
    return left.revision === right.revision && left.value === right.value;
  }

  #clearQueued(field: NoteField): void {
    this.#fieldState(field).queued = null;
    this.#queuedFields = this.#queuedFields.filter((queuedField) => queuedField !== field);
  }

  #clearAllQueued(): void {
    this.#contentState.queued = null;
    this.#colorState.queued = null;
    this.#queuedFields = [];
  }

  #queueSave(field: NoteField, pending: PendingSave): void {
    const fieldState = this.#fieldState(field);
    if (fieldState.blockedRevision === pending.revision) return;
    if (this.#activeSave?.field === field && this.#sameSave(this.#activeSave, pending)) return;
    fieldState.queued = pending;
    if (!this.#queuedFields.includes(field)) this.#queuedFields.push(field);
  }

  #startNextQueued(): void {
    while (this.#queuedFields.length > 0) {
      const field = this.#queuedFields.shift();
      if (field === undefined) return;
      const fieldState = this.#fieldState(field);
      const pending = fieldState.queued;
      fieldState.queued = null;
      if (!pending) continue;
      this.#startSave(field, pending);
      return;
    }
  }

  #schedule(field: NoteField): void {
    const fieldState = this.#fieldState(field);
    this.#clearDebounce(fieldState);
    fieldState.debounceTimer = setTimeout(() => {
      fieldState.debounceTimer = null;
      const pending = { revision: fieldState.revision, value: this.#draft(field), onSaved: fieldState.onSaved };
      if (this.#activeSave) {
        this.#queueSave(field, pending);
      } else {
        this.#startSave(field, pending);
      }
    }, DEBOUNCE_MS[field]);
  }

  #setField(field: NoteField, value: string, onSaved?: () => void): void {
    if (this.#disposed || this.#draft(field) === value) return;

    const fieldState = this.#fieldState(field);
    fieldState.revision += 1;
    fieldState.onSaved = onSaved;
    this.#clearQueued(field);
    this.#setDraft(field, value);

    if (this.#activeSave?.field !== field && value === fieldState.baseline) {
      this.#clearDebounce(fieldState);
      fieldState.dirty = false;
      fieldState.onSaved = undefined;
      this.#setFailed(field, false);
      return;
    }

    fieldState.dirty = true;
    this.#schedule(field);
  }

  #beginRequest(): void {
    if (!this.#displayIntervalActive) {
      this.#displayIntervalActive = true;
      this.#displayDelayElapsed = false;
      this.#displayTimer = setTimeout(() => {
        this.#displayTimer = null;
        if (this.#disposed || !this.#displayIntervalActive || this.#activeSave === null) return;
        this.#displayDelayElapsed = true;
        this.#updateSaveDisplay();
      }, 500);
    }
    this.#updateSaveDisplay();
  }

  #finishRequestActivity(): void {
    if (this.#activeSave === null) {
      this.#displayIntervalActive = false;
      this.#displayDelayElapsed = false;
      if (this.#displayTimer !== null) {
        clearTimeout(this.#displayTimer);
        this.#displayTimer = null;
      }
    }
    this.#updateSaveDisplay();
  }

  #updateSaveDisplay(): void {
    if (this.#hasFailed()) {
      this.saveDisplay = 'failed';
    } else if (this.#displayDelayElapsed && this.#activeSave !== null) {
      this.saveDisplay = 'saving';
    } else {
      this.saveDisplay = 'none';
    }
  }

  #startSave(field: NoteField, pending: PendingSave): void {
    if (this.#disposed) return;

    const fieldState = this.#fieldState(field);
    if (fieldState.blockedRevision === pending.revision) return;
    if (this.#activeSave) {
      this.#queueSave(field, pending);
      return;
    }

    this.#clearQueued(field);
    this.#activeSave = { field, ...pending };
    this.#setFailed(field, false);
    this.#beginRequest();

    let result: Promise<NoteSaveOutcome>;
    try {
      result = this.#save({ field, value: pending.value });
    } catch {
      result = Promise.resolve({ kind: 'failed' });
    }
    void result.then(
      (outcome) => this.#completeSave(field, pending, outcome),
      () => this.#completeSave(field, pending, { kind: 'failed' }),
    );
  }

  #completeSave(field: NoteField, pending: PendingSave, outcome: NoteSaveOutcome): void {
    if (this.#disposed) return;
    if (this.#activeSave?.field !== field || !this.#sameSave(this.#activeSave, pending)) return;

    const fieldState = this.#fieldState(field);
    this.#activeSave = null;

    if (outcome.kind === 'saved') {
      fieldState.blockedRevision = null;
      fieldState.baseline = outcome.snapshot[field];
      if (fieldState.revision === pending.revision) {
        this.#setDraft(field, outcome.snapshot[field]);
        fieldState.dirty = false;
        fieldState.onSaved = undefined;
      }
      try {
        pending.onSaved?.();
      } catch {
        // Analytics is best-effort and must not interrupt the serialized save lane.
      }
    } else if (outcome.kind === 'failed' && fieldState.revision === pending.revision) {
      fieldState.blockedRevision = pending.revision;
      this.#setFailed(field, true);
    }

    if (outcome.kind === 'subscription_gated') {
      for (const blockedFieldState of [this.#contentState, this.#colorState]) {
        if (blockedFieldState.dirty) blockedFieldState.blockedRevision = blockedFieldState.revision;
      }
      this.#clearAllQueued();
    } else {
      this.#startNextQueued();
    }
    this.#finishRequestActivity();
  }

  sync(snapshot: NoteFieldSnapshot): void {
    if (this.#disposed) return;

    for (const field of ['content', 'color'] as const) {
      const fieldState = this.#fieldState(field);
      fieldState.baseline = snapshot[field];
      if (
        fieldState.dirty &&
        fieldState.blockedRevision === fieldState.revision &&
        this.#activeSave === null &&
        snapshot[field] === this.#draft(field)
      ) {
        this.#clearDebounce(fieldState);
        this.#clearQueued(field);
        fieldState.blockedRevision = null;
        fieldState.dirty = false;
        fieldState.onSaved = undefined;
        this.#setFailed(field, false);
      } else if (!fieldState.dirty) {
        this.#setDraft(field, snapshot[field]);
      }
    }
  }

  setContent(content: string): void {
    this.#setField('content', content);
  }

  setColor(color: string, options?: { onSaved?: () => void }): void {
    this.#setField('color', color, options?.onSaved);
  }

  flush(): void {
    if (this.#disposed) return;

    for (const field of ['content', 'color'] as const) {
      const fieldState = this.#fieldState(field);
      if (!fieldState.dirty) continue;
      this.#clearDebounce(fieldState);
      const pending = { revision: fieldState.revision, value: this.#draft(field), onSaved: fieldState.onSaved };
      if (this.#activeSave) this.#queueSave(field, pending);
      else this.#startSave(field, pending);
    }
  }

  dispose(): void {
    if (this.#disposed) return;
    this.#disposed = true;
    for (const fieldState of [this.#contentState, this.#colorState]) {
      this.#clearDebounce(fieldState);
      fieldState.dirty = false;
      fieldState.failed = false;
      fieldState.blockedRevision = null;
      fieldState.onSaved = undefined;
      fieldState.queued = null;
    }
    this.#activeSave = null;
    this.#queuedFields = [];
    if (this.#displayTimer !== null) {
      clearTimeout(this.#displayTimer);
      this.#displayTimer = null;
    }
    this.#displayIntervalActive = false;
    this.#displayDelayElapsed = false;
    this.saveDisplay = 'none';
  }
}

export class NoteEdits {
  readonly #entries = new SvelteMap<string, { siteId: string; state: NoteEditState }>();
  readonly #options: NoteEditsOptions;

  constructor(options: NoteEditsOptions) {
    this.#options = options;
  }

  get(noteId: string, initialSnapshot: NoteFieldSnapshot, siteId: string): NoteEditState | undefined {
    let existing = this.#entries.get(noteId);
    if (existing && existing.siteId !== siteId) {
      existing.state.dispose();
      this.#entries.delete(noteId);
      existing = undefined;
    }
    if (this.#options.isTerminallyDeleted(siteId, noteId)) {
      this.remove(noteId);
      return undefined;
    }

    if (existing) {
      existing.state.sync(initialSnapshot);
      return existing.state;
    }

    const state = new NoteEditState({
      initialSnapshot,
      save: async (request) => {
        const outcome = await this.#options.save({ ...request, siteId, noteId });
        if (outcome.kind === 'not_found') {
          const current = this.#entries.get(noteId);
          if (!current || current.state === state) this.remove(noteId);
        }
        return outcome;
      },
    });
    this.#entries.set(noteId, { siteId, state });
    return state;
  }

  sync(noteId: string, snapshot: NoteFieldSnapshot): NoteEditState | undefined {
    return untrack(() => {
      const entry = this.#entries.get(noteId);
      if (!entry) return;
      if (this.#options.isTerminallyDeleted(entry.siteId, noteId)) {
        this.remove(noteId);
        return;
      }
      entry.state.sync(snapshot);
      return entry.state;
    });
  }

  remove(noteId: string): void {
    const entry = this.#entries.get(noteId);
    entry?.state.dispose();
    this.#entries.delete(noteId);
  }
}

const NOTE_EDITS_CONTEXT = Symbol('NoteEdits');

export function setNoteEditsContext(noteEdits: NoteEdits): NoteEdits {
  setContext(NOTE_EDITS_CONTEXT, noteEdits);
  return noteEdits;
}

export function getNoteEditsContext(): NoteEdits {
  const noteEdits = getContext<NoteEdits>(NOTE_EDITS_CONTEXT);
  if (!noteEdits) {
    throw new Error('NoteEdits not found');
  }
  return noteEdits;
}
