import { AggregatedError } from '@mearie/svelte';
import { TypieError } from '@typie/lib/errors';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteOperations } from './note-mutation';
import { NoteSync } from './note-sync.svelte';

type NoteMutationOptions = NonNullable<Parameters<NoteOperations['update']>[1]>;
type NoteMutationAnalytics = NonNullable<NoteMutationOptions['analytics']>;
type NoteMutationTarget = NonNullable<NoteMutationOptions['lastKnown']>;
type NoteSnapshot = Extract<Awaited<ReturnType<NoteOperations['update']>>, { status: 'success' }>['value'];

const { captureException, createMutation } = vi.hoisted(() => ({
  captureException: vi.fn(),
  createMutation: vi.fn(),
}));

vi.mock('@sentry/sveltekit', () => ({ captureException }));
vi.mock('@mearie/svelte', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@mearie/svelte')>()),
  createMutation,
}));

beforeEach(() => {
  captureException.mockReset();
});

const snapshot = (overrides: Partial<NoteSnapshot> = {}): NoteSnapshot => ({
  id: 'note-1',
  content: 'content',
  color: 'gray',
  order: 'a0',
  status: 'OPEN',
  updatedAt: '2026-07-29T00:00:00.000Z',
  site: { id: 'site-1' },
  ...overrides,
});

function createHarness(options: { admit?: (via: string) => boolean; mutationError?: unknown } = {}) {
  const invalidateGlobal = vi.fn();
  const invalidateEntity = vi.fn();
  const sync = new NoteSync({
    invalidateGlobal,
    invalidateEntity,
  });
  const terminalListener = vi.fn();
  sync.onTerminalDelete({ siteId: 'site-1', listener: terminalListener });
  const publishLocal = vi.spyOn(sync, 'publishLocal');

  const mutation = () =>
    vi.fn<(input: unknown) => Promise<NoteSnapshot>>(async () => {
      if (options.mutationError) throw options.mutationError;
      return snapshot();
    });
  const mutations = {
    create: mutation(),
    update: mutation(),
    move: mutation(),
    delete: mutation(),
    addEntity: mutation(),
    removeEntity: mutation(),
  };
  createMutation.mockReset();
  createMutation
    .mockReturnValueOnce([vi.fn(async ({ input }: { input: unknown }) => ({ createNote: await mutations.create(input) }))])
    .mockReturnValueOnce([vi.fn(async ({ input }: { input: unknown }) => ({ updateNote: await mutations.update(input) }))])
    .mockReturnValueOnce([vi.fn(async ({ input }: { input: unknown }) => ({ moveNote: await mutations.move(input) }))])
    .mockReturnValueOnce([vi.fn(async ({ input }: { input: unknown }) => ({ deleteNote: await mutations.delete(input) }))])
    .mockReturnValueOnce([vi.fn(async ({ input }: { input: unknown }) => ({ addNoteEntity: await mutations.addEntity(input) }))])
    .mockReturnValueOnce([vi.fn(async ({ input }: { input: unknown }) => ({ removeNoteEntity: await mutations.removeEntity(input) }))]);
  const admit = vi.fn(options.admit ?? (() => true));
  const operations = new NoteOperations({
    clientId: 'client-1',
    sync,
    admit,
  });

  return { operations, sync, mutations, admit, invalidateGlobal, invalidateEntity, terminalListener, publishLocal };
}

describe('NoteOperations successful mutations', () => {
  it.each([
    {
      name: 'create',
      invoke: (operations: NoteOperations, analytics: NoteMutationAnalytics) =>
        operations.create({ content: 'content', color: 'gray', siteId: 'site-1' }, { analytics }),
      mutation: 'create' as const,
      expectedInput: { content: 'content', color: 'gray', siteId: 'site-1', clientId: 'client-1' },
      via: 'notes_create',
      kind: 'CREATED',
    },
    {
      name: 'update',
      invoke: (operations: NoteOperations, analytics: NoteMutationAnalytics) =>
        operations.update({ noteId: 'note-1', content: 'changed' }, { analytics }),
      mutation: 'update' as const,
      expectedInput: { noteId: 'note-1', content: 'changed', clientId: 'client-1' },
      via: 'notes_update',
      kind: 'UPDATED',
    },
    {
      name: 'delete',
      invoke: (operations: NoteOperations, analytics: NoteMutationAnalytics) => operations.delete({ noteId: 'note-1' }, { analytics }),
      mutation: 'delete' as const,
      expectedInput: { noteId: 'note-1', clientId: 'client-1' },
      via: null,
      kind: 'DELETED',
    },
    {
      name: 'addEntity',
      invoke: (operations: NoteOperations, analytics: NoteMutationAnalytics) =>
        operations.addEntity({ noteId: 'note-1', entityId: 'entity-2' }, { analytics }),
      mutation: 'addEntity' as const,
      expectedInput: { noteId: 'note-1', entityId: 'entity-2', clientId: 'client-1' },
      via: 'notes_add_entity',
      kind: 'UPDATED',
    },
    {
      name: 'removeEntity',
      invoke: (operations: NoteOperations, analytics: NoteMutationAnalytics) =>
        operations.removeEntity({ noteId: 'note-1', entityId: 'entity-2' }, { analytics }),
      mutation: 'removeEntity' as const,
      expectedInput: { noteId: 'note-1', entityId: 'entity-2', clientId: 'client-1' },
      via: 'notes_remove_entity',
      kind: 'UPDATED',
    },
  ])('normalizes $name success, adds clientId, publishes one wildcard update, and calls success analytics once', async (testCase) => {
    const { operations, mutations, admit, invalidateGlobal, terminalListener, publishLocal } = createHarness();
    const analytics = { onSuccess: vi.fn<(value: NoteSnapshot) => void>() };

    const result = await testCase.invoke(operations, analytics);

    expect(result).toEqual({ status: 'success', value: snapshot() });
    expect(mutations[testCase.mutation]).toHaveBeenCalledOnce();
    expect(mutations[testCase.mutation]).toHaveBeenCalledWith(testCase.expectedInput);
    if (testCase.via) expect(admit.mock.calls).toEqual([[testCase.via]]);
    else expect(admit).not.toHaveBeenCalled();
    expect(publishLocal).toHaveBeenCalledOnce();
    expect(publishLocal).toHaveBeenCalledWith({
      kind: testCase.kind,
      noteId: 'note-1',
      siteId: 'site-1',
    });
    expect(invalidateGlobal.mock.calls).toEqual([['site-1']]);
    expect(analytics.onSuccess).toHaveBeenCalledOnce();
    expect(analytics.onSuccess).toHaveBeenCalledWith(snapshot());
    expect(terminalListener).toHaveBeenCalledTimes(testCase.kind === 'DELETED' ? 1 : 0);
  });

  it('uses the move response without invalidating note queries', async () => {
    const { operations, mutations, admit, invalidateGlobal, publishLocal } = createHarness();
    const analytics = { onSuccess: vi.fn<(value: NoteSnapshot) => void>() };

    const result = await operations.move({ noteId: 'note-1', lowerOrder: 'a0' }, { analytics });

    expect(result).toEqual({ status: 'success', value: snapshot() });
    expect(mutations.move).toHaveBeenCalledWith({ noteId: 'note-1', lowerOrder: 'a0', clientId: 'client-1' });
    expect(admit.mock.calls).toEqual([['notes_move']]);
    expect(publishLocal).not.toHaveBeenCalled();
    expect(invalidateGlobal).not.toHaveBeenCalled();
    expect(analytics.onSuccess).toHaveBeenCalledWith(snapshot());
  });

  it('keeps a successful outcome after best-effort success analytics throws', async () => {
    const { operations, publishLocal } = createHarness();
    const onSuccess = vi.fn<(value: NoteSnapshot) => void>(() => {
      throw new Error('analytics failed');
    });

    await expect(operations.update({ noteId: 'note-1', content: 'changed' }, { analytics: { onSuccess } })).resolves.toEqual({
      status: 'success',
      value: snapshot(),
    });

    expect(onSuccess).toHaveBeenCalledOnce();
    expect(publishLocal).toHaveBeenCalledOnce();
    expect(publishLocal).toHaveBeenCalledWith({
      kind: 'UPDATED',
      noteId: 'note-1',
      siteId: 'site-1',
    });
  });

  it('keeps a successful outcome when local cache invalidation throws after the mutation commits', async () => {
    const { operations, publishLocal } = createHarness();
    const error = new Error('cache unavailable');
    publishLocal.mockImplementationOnce(() => {
      throw error;
    });

    await expect(operations.update({ noteId: 'note-1', content: 'changed' })).resolves.toEqual({
      status: 'success',
      value: snapshot(),
    });

    expect(captureException).toHaveBeenCalledOnce();
    expect(captureException).toHaveBeenCalledWith(error);
  });
});

describe('NoteOperations rejected and failed mutations', () => {
  it('returns subscription_gated without calling a mutation or publishing when admission rejects an operation', async () => {
    const { operations, mutations, invalidateGlobal } = createHarness({ admit: () => false });

    const result = await operations.update({ noteId: 'note-1', content: 'changed' });

    expect(result).toEqual({ status: 'subscription_gated' });
    expect(captureException).not.toHaveBeenCalled();
    expect(mutations.update).not.toHaveBeenCalled();
    expect(invalidateGlobal).not.toHaveBeenCalled();
  });

  it('normalizes a thrown admission check as failure without calling the mutation', async () => {
    const error = new Error('admission unavailable');
    const { operations, mutations, invalidateGlobal } = createHarness({
      admit: () => {
        throw error;
      },
    });

    const result = await operations.addEntity({ noteId: 'note-1', entityId: 'entity-1' });

    expect(result).toEqual({ status: 'failure', error });
    expect(captureException).toHaveBeenCalledOnce();
    expect(captureException).toHaveBeenCalledWith(error);
    expect(mutations.addEntity).not.toHaveBeenCalled();
    expect(invalidateGlobal).not.toHaveBeenCalled();
  });

  it('keeps ordinary mutation errors as failures without invalidation or tombstones', async () => {
    const error = new Error('offline');
    const { operations, sync, invalidateGlobal } = createHarness({ mutationError: error });

    const result = await operations.delete({ noteId: 'note-1' }, { lastKnown: { siteId: 'site-1', noteId: 'note-1' } });

    expect(result).toEqual({ status: 'failure', error });
    expect(captureException).toHaveBeenCalledOnce();
    expect(captureException).toHaveBeenCalledWith(error);
    expect(invalidateGlobal).not.toHaveBeenCalled();
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(false);
  });

  it('keeps an ordinary failure outcome when error reporting throws', async () => {
    const error = new Error('offline');
    captureException.mockImplementationOnce(() => {
      throw new Error('reporting unavailable');
    });
    const { operations } = createHarness({ mutationError: error });

    await expect(operations.update({ noteId: 'note-1', content: 'changed' })).resolves.toEqual({ status: 'failure', error });
  });
});

describe('NoteOperations not_found handling', () => {
  const notFound = () => new AggregatedError([new TypieError({ code: 'not_found' }) as never]);

  it('marks a targeted not_found as terminally deleted and calls terminal analytics at most once', async () => {
    const { operations, sync, invalidateGlobal, invalidateEntity, terminalListener } = createHarness({ mutationError: notFound() });
    const terminalAnalytics = vi.fn<(target: NoteMutationTarget) => void>();
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });

    const options = {
      lastKnown: { siteId: 'site-1', noteId: 'note-1' },
      analytics: { onTerminal: terminalAnalytics },
    };
    const first = await operations.update({ noteId: 'note-1', content: 'changed' }, options);
    const second = await operations.update({ noteId: 'note-1', content: 'changed again' }, options);

    expect(first).toEqual({ status: 'not_found' });
    expect(second).toEqual({ status: 'not_found' });
    expect(captureException).not.toHaveBeenCalled();
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);
    expect(invalidateGlobal.mock.calls).toEqual([['site-1']]);
    expect(invalidateEntity.mock.calls).toEqual([['site-1', 'entity-1']]);
    expect(terminalListener).toHaveBeenCalledOnce();
    expect(terminalAnalytics).toHaveBeenCalledOnce();
    expect(terminalAnalytics).toHaveBeenCalledWith(options.lastKnown);
  });

  it('keeps not_found after best-effort terminal analytics throws', async () => {
    const { operations, sync, invalidateGlobal, invalidateEntity } = createHarness({ mutationError: notFound() });
    const onTerminal = vi.fn<(target: NoteMutationTarget) => void>(() => {
      throw new Error('analytics failed');
    });
    sync.retainRelatedEntity({ siteId: 'site-1', entityId: 'entity-1' });

    await expect(
      operations.update(
        { noteId: 'note-1', content: 'changed' },
        {
          lastKnown: { siteId: 'site-1', noteId: 'note-1' },
          analytics: { onTerminal },
        },
      ),
    ).resolves.toEqual({ status: 'not_found' });

    expect(onTerminal).toHaveBeenCalledOnce();
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);
    expect(invalidateGlobal.mock.calls).toEqual([['site-1']]);
    expect(invalidateEntity.mock.calls).toEqual([['site-1', 'entity-1']]);
  });

  it('keeps not_found after terminal cache invalidation throws', async () => {
    const { operations, sync, invalidateGlobal } = createHarness({ mutationError: notFound() });
    const error = new Error('cache unavailable');
    invalidateGlobal.mockImplementationOnce(() => {
      throw error;
    });

    await expect(
      operations.update(
        { noteId: 'note-1', content: 'changed' },
        {
          lastKnown: { siteId: 'site-1', noteId: 'note-1' },
        },
      ),
    ).resolves.toEqual({ status: 'not_found' });

    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(true);
    expect(captureException).toHaveBeenCalledOnce();
    expect(captureException).toHaveBeenCalledWith(error);
  });

  it('returns not_found without recording a terminal deletion when no last-known target exists', async () => {
    const { operations, sync, invalidateGlobal } = createHarness({ mutationError: notFound() });

    const result = await operations.move({ noteId: 'note-1', upperOrder: 'b0' });

    expect(result).toEqual({ status: 'not_found' });
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(false);
    expect(invalidateGlobal).not.toHaveBeenCalled();
  });

  it('keeps create not_found as an ordinary failure without a tombstone', async () => {
    const error = notFound();
    const { operations, sync, invalidateGlobal } = createHarness({ mutationError: error });

    const result = await operations.create({ content: 'content', color: 'gray', siteId: 'site-1' });

    expect(result).toEqual({ status: 'failure', error });
    expect(captureException).toHaveBeenCalledOnce();
    expect(sync.isTerminallyDeleted('site-1', 'note-1')).toBe(false);
    expect(invalidateGlobal).not.toHaveBeenCalled();
  });
});
