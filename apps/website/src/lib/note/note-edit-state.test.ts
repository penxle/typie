import { mount, tick, unmount } from 'svelte';
import { SvelteMap } from 'svelte/reactivity';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteEdits, NoteEditState } from './note-edit-state.svelte';
import NoteEditsReactivityTestHost from './note-edits-reactivity-test-host.svelte';
import { NoteSync } from './note-sync.svelte';

type NoteEditStateOptions = ConstructorParameters<typeof NoteEditState>[0];
type NoteFieldSave = Parameters<NoteEditStateOptions['save']>[0];
type NoteFieldSnapshot = NoteEditStateOptions['initialSnapshot'];
type NoteSaveOutcome = Awaited<ReturnType<NoteEditStateOptions['save']>>;

const toastError = vi.hoisted(() => vi.fn());

vi.mock('@typie/ui/notification', () => ({
  Toast: { error: toastError },
}));

type Deferred<T> = {
  promise: Promise<T>;
  resolve: (value: T) => void;
};

const snapshot = (overrides: Partial<NoteFieldSnapshot> = {}): NoteFieldSnapshot => ({
  content: 'server content',
  color: 'gray',
  ...overrides,
});

const saved = (value: NoteFieldSnapshot): NoteSaveOutcome => ({ kind: 'saved', snapshot: value });
const neverTerminal = () => false;

function deferred<T>(): Deferred<T> {
  const { promise, resolve } = Promise.withResolvers<T>();
  return { promise, resolve };
}

function createState(save?: (request: NoteFieldSave) => Promise<NoteSaveOutcome>) {
  const saveField =
    save ?? vi.fn(async ({ field, value }: NoteFieldSave) => saved(snapshot(field === 'content' ? { content: value } : { color: value })));
  const state = new NoteEditState({ initialSnapshot: snapshot(), save: saveField });
  return { state, save: saveField };
}

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

beforeEach(() => {
  vi.useFakeTimers();
  toastError.mockClear();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('NoteEditState snapshot and draft ownership', () => {
  it('lets clean fields follow the newest server snapshot', () => {
    const { state } = createState();

    state.sync(snapshot({ content: 'remote content', color: 'blue' }));

    expect(state.content).toBe('remote content');
    expect(state.color).toBe('blue');
  });

  it('preserves dirty content across a remote content snapshot while clean color follows it', () => {
    const { state } = createState();
    state.setContent('local content');

    state.sync(snapshot({ content: 'remote content', color: 'blue' }));

    expect(state.content).toBe('local content');
    expect(state.color).toBe('blue');
  });

  it('preserves dirty color across a remote color snapshot while clean content follows it', () => {
    const { state } = createState();
    state.setColor('red');

    state.sync(snapshot({ content: 'remote content', color: 'blue' }));

    expect(state.content).toBe('remote content');
    expect(state.color).toBe('red');
  });

  it('does not let save A completion clear a newer content draft B', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { state } = createState(save);

    state.setContent('draft A');
    await vi.advanceTimersByTimeAsync(300);
    state.setContent('draft B');
    await vi.advanceTimersByTimeAsync(300);

    first.resolve(saved(snapshot({ content: 'draft A' })));
    await settle();

    expect(state.content).toBe('draft B');
    expect(save).toHaveBeenCalledTimes(2);
    expect(save.mock.calls[1]?.[0]).toEqual({ field: 'content', value: 'draft B' });

    second.resolve(saved(snapshot({ content: 'draft B' })));
    await settle();
  });

  it('shows later remote content after the latest draft saves and becomes clean', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const { state } = createState(() => completion.promise);
    state.setContent('draft B');
    await vi.advanceTimersByTimeAsync(300);

    completion.resolve(saved(snapshot({ content: 'draft B' })));
    await settle();
    state.sync(snapshot({ content: 'remote after B' }));

    expect(state.content).toBe('remote after B');
  });

  it('shares one registry state and synchronizes an existing clean entry', () => {
    const edits = new NoteEdits({
      isTerminallyDeleted: neverTerminal,
      save: async ({ field, value }) => saved(snapshot(field === 'content' ? { content: value } : { color: value })),
    });

    const first = edits.get('note-1', snapshot(), 'site-1');
    const second = edits.get('note-1', snapshot({ content: 'remote content', color: 'blue' }), 'site-1');

    expect(first).toBeDefined();
    expect(second).toBe(first);
    expect(first?.content).toBe('remote content');
    expect(first?.color).toBe('blue');
  });

  it('captures an explicitly requested site for related-note saves', async () => {
    const save = vi.fn(async () => saved(snapshot({ color: 'blue' })));
    const edits = new NoteEdits({
      isTerminallyDeleted: neverTerminal,
      save,
    });

    const state = edits.get('note-1', snapshot(), 'related-site');
    state?.setColor('blue');
    await vi.advanceTimersByTimeAsync(180);

    expect(save).toHaveBeenCalledWith({
      siteId: 'related-site',
      noteId: 'note-1',
      field: 'color',
      value: 'blue',
    });
  });

  it('replaces a registry entry when the same note id is opened for another site', async () => {
    const save = vi.fn(async ({ field, value }) => saved(snapshot(field === 'content' ? { content: value } : { color: value })));
    const edits = new NoteEdits({
      isTerminallyDeleted: neverTerminal,
      save,
    });
    const first = edits.get('note-1', snapshot(), 'site-1');
    const second = edits.get('note-1', snapshot(), 'site-2');

    expect(second).not.toBe(first);
    second?.setColor('blue');
    await vi.advanceTimersByTimeAsync(180);

    expect(save).toHaveBeenCalledWith({
      siteId: 'site-2',
      noteId: 'note-1',
      field: 'color',
      value: 'blue',
    });
  });

  it('removes an existing entry and refuses stale get and sync when its captured site is tombstoned', async () => {
    let terminal = false;
    const save = vi.fn(async () => saved(snapshot({ color: 'blue' })));
    const edits = new NoteEdits({
      isTerminallyDeleted: () => terminal,
      save,
    });
    const state = edits.get('note-1', snapshot(), 'site-1');
    expect(state).toBeDefined();
    state?.setColor('blue');

    terminal = true;

    expect(edits.get('note-1', snapshot({ color: 'stale' }), 'site-1')).toBeUndefined();
    expect(edits.sync('note-1', snapshot({ color: 'also stale' }))).toBeUndefined();
    await vi.advanceTimersByTimeAsync(180);
    expect(save).not.toHaveBeenCalled();
  });

  it('removes structurally by cancelling pending work and invalidating older in-flight callbacks', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const save = vi.fn(() => completion.promise);
    const edits = new NoteEdits({ isTerminallyDeleted: neverTerminal, save });
    const state = edits.get('note-1', snapshot(), 'site-1');
    expect(state).toBeDefined();
    state?.setContent('in flight');
    state?.setColor('still debounced');
    await vi.advanceTimersByTimeAsync(180);

    edits.remove('note-1');
    await vi.advanceTimersByTimeAsync(1000);
    completion.resolve(saved(snapshot({ content: 'in flight', color: 'blue' })));
    await settle();
    state?.setColor('resurrection attempt');
    state?.flush();
    await vi.advanceTimersByTimeAsync(1000);

    expect(save).toHaveBeenCalledOnce();
    expect(edits.sync('note-1', snapshot({ content: 'resurrected' }))).toBeUndefined();
    expect(state?.saveDisplay).toBe('none');
  });

  it('tracks snapshot changes but not registry insertion or removal through sync', async () => {
    const edits = new NoteEdits({
      isTerminallyDeleted: neverTerminal,
      save: async () => saved(snapshot()),
    });
    const reactiveSnapshot = new SvelteMap([
      ['content', 'server content'],
      ['color', 'gray'],
    ]);
    const onRun = vi.fn();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteEditsReactivityTestHost, {
      target,
      props: { noteEdits: edits, noteId: 'note-1', snapshot: reactiveSnapshot, onRun },
    });

    try {
      await tick();
      expect(onRun).toHaveBeenCalledOnce();

      const state = edits.get('note-1', snapshot(), 'site-1');
      await tick();
      expect(onRun).toHaveBeenCalledOnce();

      reactiveSnapshot.set('content', 'remote content');
      reactiveSnapshot.set('color', 'blue');
      await tick();
      expect(onRun).toHaveBeenCalledTimes(2);
      expect(state?.content).toBe('remote content');
      expect(state?.color).toBe('blue');

      edits.remove('note-1');
      await tick();
      expect(onRun).toHaveBeenCalledTimes(2);
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('does not rerun a reactive sync caller when terminal cleanup removes the entry', async () => {
    let terminal = false;
    const edits = new NoteEdits({
      isTerminallyDeleted: () => terminal,
      save: async () => saved(snapshot()),
    });
    edits.get('note-1', snapshot(), 'site-1');
    const reactiveSnapshot = new SvelteMap([
      ['content', 'server content'],
      ['color', 'gray'],
    ]);
    const onRun = vi.fn();
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteEditsReactivityTestHost, {
      target,
      props: { noteEdits: edits, noteId: 'note-1', snapshot: reactiveSnapshot, onRun },
    });

    try {
      await tick();
      expect(onRun).toHaveBeenCalledOnce();

      terminal = true;
      expect(edits.sync('note-1', snapshot())).toBeUndefined();
      await tick();

      expect(onRun).toHaveBeenCalledOnce();
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});

describe('NoteEditState per-field debounce and serialization', () => {
  it('debounces color for 180ms and content for 300ms', async () => {
    const { state, save } = createState();
    state.setContent('content draft');
    state.setColor('blue');

    await vi.advanceTimersByTimeAsync(179);
    expect(save).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledWith({ field: 'color', value: 'blue' });
    await vi.advanceTimersByTimeAsync(119);
    expect(save).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ field: 'content', value: 'content draft' });
  });

  it('replaces only the edited field debounce job', async () => {
    const { state, save } = createState();
    state.setContent('content A');
    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(100);
    state.setContent('content B');

    await vi.advanceTimersByTimeAsync(80);
    expect(save).toHaveBeenCalledOnce();
    expect(save).toHaveBeenCalledWith({ field: 'color', value: 'blue' });
    await vi.advanceTimersByTimeAsync(219);
    expect(save).toHaveBeenCalledOnce();
    await vi.advanceTimersByTimeAsync(1);
    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ field: 'content', value: 'content B' });
  });

  it('serializes content saves with content saves', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { state } = createState(save);
    state.setContent('content A');
    await vi.advanceTimersByTimeAsync(300);
    state.setContent('content B');
    await vi.advanceTimersByTimeAsync(300);

    expect(save).toHaveBeenCalledOnce();

    first.resolve(saved(snapshot({ content: 'content A' })));
    await settle();

    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ field: 'content', value: 'content B' });
    second.resolve(saved(snapshot({ content: 'content B' })));
    await settle();
  });

  it('serializes color saves with color saves', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { state } = createState(save);
    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(180);
    state.setColor('red');
    await vi.advanceTimersByTimeAsync(180);

    expect(save).toHaveBeenCalledOnce();

    first.resolve(saved(snapshot({ color: 'blue' })));
    await settle();

    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ field: 'color', value: 'red' });
    second.resolve(saved(snapshot({ color: 'red' })));
    await settle();
  });

  it('reports each successful color revision to the callback that created it', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const firstSaved = vi.fn();
    const secondSaved = vi.fn();
    const { state } = createState(save);

    state.setColor('blue', { onSaved: firstSaved });
    await vi.advanceTimersByTimeAsync(180);
    state.setColor('red', { onSaved: secondSaved });
    await vi.advanceTimersByTimeAsync(180);

    first.resolve(saved(snapshot({ color: 'blue' })));
    await settle();
    expect(firstSaved).toHaveBeenCalledOnce();
    expect(secondSaved).not.toHaveBeenCalled();

    second.resolve(saved(snapshot({ color: 'red' })));
    await settle();
    expect(secondSaved).toHaveBeenCalledOnce();
  });

  it.each([{ kind: 'failed' as const }, { kind: 'subscription_gated' as const }, { kind: 'not_found' as const }])(
    'does not report a color revision after $kind',
    async (outcome) => {
      const onSaved = vi.fn();
      const { state } = createState(vi.fn().mockResolvedValue(outcome));

      state.setColor('blue', { onSaved });
      await vi.advanceTimersByTimeAsync(180);
      await settle();

      expect(onSaved).not.toHaveBeenCalled();
    },
  );

  it('isolates a color success callback exception and starts the next queued save', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const secondSaved = vi.fn();
    const { state } = createState(save);

    state.setColor('blue', {
      onSaved: () => {
        throw new Error('analytics failed');
      },
    });
    await vi.advanceTimersByTimeAsync(180);
    state.setColor('red', { onSaved: secondSaved });
    await vi.advanceTimersByTimeAsync(180);

    first.resolve(saved(snapshot({ color: 'blue' })));
    await settle();
    expect(save).toHaveBeenCalledTimes(2);

    second.resolve(saved(snapshot({ color: 'red' })));
    await settle();
    expect(secondSaved).toHaveBeenCalledOnce();
  });

  it('serializes content and color saves so full snapshots cannot arrive out of order', async () => {
    const completions = [deferred<NoteSaveOutcome>(), deferred<NoteSaveOutcome>()];
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(completions[0].promise)
      .mockReturnValueOnce(completions[1].promise);
    const { state } = createState(save);
    state.setContent('content draft');
    state.setColor('blue');

    await vi.advanceTimersByTimeAsync(300);

    expect(save).toHaveBeenCalledOnce();
    expect(save).toHaveBeenCalledWith({ field: 'color', value: 'blue' });

    completions[0].resolve(saved(snapshot({ color: 'blue' })));
    await settle();

    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ field: 'content', value: 'content draft' });

    completions[1].resolve(saved(snapshot({ content: 'content draft' })));
    await settle();
  });

  it('updates only the saved field baseline from each returned snapshot', async () => {
    const colorCompletion = deferred<NoteSaveOutcome>();
    const contentCompletion = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(colorCompletion.promise)
      .mockReturnValueOnce(contentCompletion.promise);
    const { state } = createState(save);
    state.setContent('local content');
    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(300);

    colorCompletion.resolve(saved({ content: 'wrong content', color: 'blue' }));
    await settle();
    state.sync({ content: 'remote while content is dirty', color: 'remote after color' });

    expect(state.content).toBe('local content');
    expect(state.color).toBe('remote after color');

    contentCompletion.resolve(saved({ content: 'local content', color: 'wrong color' }));
    await settle();
    state.sync({ content: 'latest remote content', color: 'remote after color' });

    expect(state.content).toBe('latest remote content');
    expect(state.color).toBe('remote after color');
  });

  it('flushes both pending fields immediately without duplicate debounce saves', async () => {
    const { state, save } = createState();
    const mockedSave = vi.mocked(save);
    state.setContent('content draft');
    state.setColor('blue');

    state.flush();
    expect(mockedSave).toHaveBeenCalledOnce();
    expect(mockedSave).toHaveBeenCalledWith({ field: 'content', value: 'content draft' });

    await settle();
    expect(mockedSave).toHaveBeenCalledTimes(2);
    expect(mockedSave.mock.calls.map(([request]) => request)).toEqual([
      { field: 'content', value: 'content draft' },
      { field: 'color', value: 'blue' },
    ]);

    await vi.advanceTimersByTimeAsync(1000);
    expect(mockedSave).toHaveBeenCalledTimes(2);
  });

  it('does not queue duplicate flushes for an active revision but still serializes a genuinely newer revision', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const third = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise)
      .mockReturnValueOnce(third.promise);
    const { state } = createState(save);
    state.setContent('revision A');
    await vi.advanceTimersByTimeAsync(300);

    state.flush();
    state.flush();
    expect(save).toHaveBeenCalledOnce();

    first.resolve(saved(snapshot({ content: 'revision A' })));
    await settle();

    expect(save).toHaveBeenCalledOnce();
    expect(state.saveDisplay).toBe('none');

    state.setContent('revision B');
    await vi.advanceTimersByTimeAsync(300);
    state.setContent('revision C');
    state.flush();
    state.flush();
    expect(save).toHaveBeenCalledTimes(2);

    second.resolve(saved(snapshot({ content: 'revision B' })));
    await settle();
    expect(save).toHaveBeenCalledTimes(3);
    expect(save).toHaveBeenLastCalledWith({ field: 'content', value: 'revision C' });

    third.resolve(saved(snapshot({ content: 'revision C' })));
    await settle();
  });

  it('saves a compensating baseline value after an in-flight revision fails ambiguously', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { state } = createState(save);

    state.setContent('remote revision');
    await vi.advanceTimersByTimeAsync(300);
    state.setContent('server content');
    await vi.advanceTimersByTimeAsync(300);
    expect(save).toHaveBeenCalledOnce();

    first.resolve({ kind: 'failed' });
    await settle();

    expect(save).toHaveBeenCalledTimes(2);
    expect(save).toHaveBeenLastCalledWith({ field: 'content', value: 'server content' });

    second.resolve(saved(snapshot()));
    await settle();
  });
});

describe('NoteEditState delayed save display and failures', () => {
  it('starts the 500ms saving delay when the request starts, not during debounce', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const { state } = createState(() => completion.promise);
    state.setContent('content draft');

    await vi.advanceTimersByTimeAsync(799);
    expect(state.saveDisplay).toBe('none');
    await vi.advanceTimersByTimeAsync(1);
    expect(state.saveDisplay).toBe('saving');

    completion.resolve(saved(snapshot({ content: 'content draft' })));
    await settle();
  });

  it('does not restart the aggregate interval for an overlapping field and stays active when one completes', async () => {
    const colorCompletion = deferred<NoteSaveOutcome>();
    const contentCompletion = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(colorCompletion.promise)
      .mockReturnValueOnce(contentCompletion.promise);
    const { state } = createState(save);
    state.setContent('content draft');
    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(500);

    colorCompletion.resolve(saved(snapshot({ color: 'blue' })));
    await settle();
    expect(state.saveDisplay).toBe('none');

    await vi.advanceTimersByTimeAsync(179);
    expect(state.saveDisplay).toBe('none');
    await vi.advanceTimersByTimeAsync(1);
    expect(state.saveDisplay).toBe('saving');

    contentCompletion.resolve(saved(snapshot({ content: 'content draft' })));
    await settle();
  });

  it('retains an ordinary failed field draft and reports failed', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const { state } = createState(() => completion.promise);
    state.setContent('content draft');
    await vi.advanceTimersByTimeAsync(300);

    completion.resolve({ kind: 'failed' });
    await settle();

    expect(state.content).toBe('content draft');
    expect(state.saveDisplay).toBe('failed');
  });

  it('keeps the latest draft without failure or automatic replay when saving is subscription gated', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const save = vi.fn(() => completion.promise);
    const { state } = createState(save);
    state.setContent('draft A');
    await vi.advanceTimersByTimeAsync(300);
    state.setContent('draft B');
    await vi.advanceTimersByTimeAsync(300);

    completion.resolve({ kind: 'subscription_gated' });
    await settle();
    await vi.advanceTimersByTimeAsync(1000);

    expect(state.content).toBe('draft B');
    expect(state.saveDisplay).toBe('none');
    expect(save).toHaveBeenCalledOnce();
    expect(toastError).not.toHaveBeenCalled();
  });

  it.each([{ kind: 'failed' as const }, { kind: 'subscription_gated' as const }])(
    'does not retry an unchanged $kind revision when focus loss flushes the draft',
    async (outcome) => {
      const save = vi.fn().mockResolvedValue(outcome);
      const { state } = createState(save);
      state.setContent('content draft');
      await vi.advanceTimersByTimeAsync(300);
      await settle();

      state.flush();
      await settle();

      expect(save).toHaveBeenCalledOnce();

      state.setContent('edited after failure');
      await vi.advanceTimersByTimeAsync(300);
      expect(save).toHaveBeenCalledTimes(2);
    },
  );

  it.each([{ kind: 'failed' as const }, { kind: 'subscription_gated' as const }])(
    'treats a matching authoritative snapshot as confirmation after $kind',
    async (outcome) => {
      const save = vi.fn().mockResolvedValue(outcome);
      const { state } = createState(save);
      state.setContent('content draft');
      await vi.advanceTimersByTimeAsync(300);
      await settle();

      state.sync(snapshot({ content: 'content draft' }));
      expect(state.saveDisplay).toBe('none');

      state.flush();
      await settle();
      expect(save).toHaveBeenCalledOnce();

      state.sync(snapshot({ content: 'later remote content' }));
      expect(state.content).toBe('later remote content');
    },
  );

  it('gives a failed field precedence while the queued field saves', async () => {
    const colorCompletion = deferred<NoteSaveOutcome>();
    const contentCompletion = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(colorCompletion.promise)
      .mockReturnValueOnce(contentCompletion.promise);
    const { state } = createState(save);
    state.setContent('content draft');
    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(680);
    expect(state.saveDisplay).toBe('saving');

    colorCompletion.resolve({ kind: 'failed' });
    await settle();

    expect(state.saveDisplay).toBe('failed');

    contentCompletion.resolve(saved(snapshot({ content: 'content draft' })));
    await settle();
  });

  it('notifies once per aggregate failure episode and clears only the field that saves again', async () => {
    const nextContentSave = deferred<NoteSaveOutcome>();
    const nextColorSave = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockResolvedValueOnce({ kind: 'failed' })
      .mockResolvedValueOnce({ kind: 'failed' })
      .mockReturnValueOnce(nextContentSave.promise)
      .mockReturnValueOnce(nextColorSave.promise);
    const { state } = createState(save);
    state.setContent('content draft');
    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(300);
    await settle();

    expect(state.content).toBe('content draft');
    expect(state.color).toBe('blue');
    expect(toastError).toHaveBeenCalledOnce();
    expect(toastError).toHaveBeenCalledWith('노트를 저장하지 못했어요.');

    state.setContent('next content draft');
    await vi.advanceTimersByTimeAsync(300);
    expect(state.saveDisplay).toBe('failed');
    expect(save).toHaveBeenCalledTimes(3);

    nextContentSave.resolve(saved(snapshot({ content: 'next content draft' })));
    await settle();

    state.setColor('red');
    await vi.advanceTimersByTimeAsync(180);
    expect(state.saveDisplay).toBe('none');
    expect(save).toHaveBeenCalledTimes(4);

    nextColorSave.resolve({ kind: 'failed' });
    await settle();

    expect(state.color).toBe('red');
    expect(state.saveDisplay).toBe('failed');
    expect(toastError).toHaveBeenCalledTimes(2);
    expect(toastError).toHaveBeenLastCalledWith('노트를 저장하지 못했어요.');
  });

  it('resets the aggregate interval after returning to zero in-flight requests', async () => {
    const first = deferred<NoteSaveOutcome>();
    const second = deferred<NoteSaveOutcome>();
    const save = vi
      .fn<(request: NoteFieldSave) => Promise<NoteSaveOutcome>>()
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);
    const { state } = createState(save);
    state.setContent('content draft');
    await vi.advanceTimersByTimeAsync(800);
    expect(state.saveDisplay).toBe('saving');

    first.resolve(saved(snapshot({ content: 'content draft' })));
    await settle();
    expect(state.saveDisplay).toBe('none');

    state.setColor('blue');
    await vi.advanceTimersByTimeAsync(679);
    expect(state.saveDisplay).toBe('none');
    await vi.advanceTimersByTimeAsync(1);
    expect(state.saveDisplay).toBe('saving');

    second.resolve(saved(snapshot({ color: 'blue' })));
    await settle();
  });
});

describe('NoteSync and NoteEdits terminal integration', () => {
  it('blocks stale get after remote delete, ignores old completion, and permits get after an explicit create', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const save = vi.fn(() => completion.promise);
    const sync = new NoteSync({
      invalidateGlobal: vi.fn(),
      invalidateEntity: vi.fn(),
    });
    const edits = new NoteEdits({
      isTerminallyDeleted: (siteId, noteId) => sync.isTerminallyDeleted(siteId, noteId),
      save,
    });
    const disposeTerminalObserver = sync.onTerminalDelete({
      siteId: 'site-1',
      listener: (noteId) => edits.remove(noteId),
    });
    const state = edits.get('note-1', snapshot(), 'site-1');
    expect(state).toBeDefined();
    state?.setContent('in flight');
    await vi.advanceTimersByTimeAsync(300);

    sync.receiveRemote({
      kind: 'DELETED',
      noteId: 'note-1',
      siteId: 'site-1',
    });
    completion.resolve(saved(snapshot({ content: 'in flight' })));
    await settle();

    expect(edits.get('note-1', snapshot({ content: 'stale query content' }), 'site-1')).toBeUndefined();
    expect(edits.sync('note-1', snapshot({ content: 'stale query content' }))).toBeUndefined();
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).toHaveBeenCalledOnce();

    sync.receiveRemote({
      kind: 'CREATED',
      noteId: 'note-1',
      siteId: 'site-1',
    });
    const recreated = edits.get('note-1', snapshot({ content: 'created again' }), 'site-1');
    expect(recreated).toBeDefined();
    expect(recreated).not.toBe(state);
    expect(recreated?.content).toBe('created again');

    disposeTerminalObserver();
  });

  it('removes a captured old-site entry on not_found after the selected-site listener has moved', async () => {
    const completion = deferred<NoteSaveOutcome>();
    const save = vi.fn(() => completion.promise);
    const sync = new NoteSync({
      invalidateGlobal: vi.fn(),
      invalidateEntity: vi.fn(),
    });
    const edits = new NoteEdits({
      isTerminallyDeleted: (siteId, noteId) => sync.isTerminallyDeleted(siteId, noteId),
      save,
    });
    const state = edits.get('note-1', snapshot(), 'site-1');
    expect(state).toBeDefined();
    state?.setContent('old-site draft');
    await vi.advanceTimersByTimeAsync(300);

    sync.markNotFound({ siteId: 'site-1', noteId: 'note-1' });
    completion.resolve({ kind: 'not_found' });
    await settle();

    expect(edits.get('note-1', snapshot({ content: 'stale old-site query' }), 'site-1')).toBeUndefined();
    state?.flush();
    await vi.advanceTimersByTimeAsync(1000);
    expect(save).toHaveBeenCalledOnce();
  });
});
