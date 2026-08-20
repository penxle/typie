import { tick, untrack } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Editor } from './editor.svelte';
import { createTrackedEffect } from './editor-effect-harness.svelte';
import { snapshot } from './registry';
import { EditorScrollScope } from './scroll.svelte';
import type { PlainRootNode, Size, TickResult, TrackedRange } from '@typie/editor-ffi/browser';

const wasmHarness = vi.hoisted(() => ({
  createEditor: vi.fn(),
  setThemeVariant: vi.fn(() => null),
}));

const sentryHarness = vi.hoisted(() => ({ captureException: vi.fn() }));

vi.mock('@sentry/sveltekit', () => sentryHarness);

vi.mock('$env/dynamic/public', () => ({ env: {} }));

vi.mock('$lib/wasm-ffi.svelte', () => ({
  initWasm: vi.fn(() => Promise.resolve()),
  wasm: {
    create_editor_from_doc: wasmHarness.createEditor,
    set_theme_variant: wasmHarness.setThemeVariant,
  },
}));

vi.mock('./gesture.svelte', () => ({
  TouchGestureController: class {
    destroy() {
      // The guarded-boundary tests do not need gesture behavior.
    }
  },
}));

vi.mock('./fonts', () => ({
  fontDataMissingHandler: vi.fn(),
}));

vi.mock('./components/ExternalElement.svelte', () => ({
  default: () => {
    // Surface lifecycle coverage does not render external-element content.
  },
}));

vi.mock('./components/LinkOverlay.svelte', () => ({
  default: () => {
    // Surface lifecycle coverage does not render link content.
  },
}));

vi.mock('./components/TableOverlay.svelte', () => ({
  default: () => {
    // Surface lifecycle coverage does not render table content.
  },
}));

type FakeCore = ReturnType<typeof createCore>;
type TerminalState = 'failed' | 'destroyed';

const DefaultPageSizes: Size[] = [
  { width: 100, height: 100 },
  { width: 100, height: 100 },
  { width: 100, height: 100 },
];

function createCore() {
  return {
    enqueue_request: vi.fn(() => ({ value: 1 })),
    tick_through: vi.fn<(requestId: { value: number }) => TickResult>((requestId) => ({
      revision: { value: 1 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }],
      request_outcomes: [{ request_id: requestId, command_outcomes: [{ type: 'applied' }] }],
    })),
    tick: vi.fn<() => TickResult | undefined>(() => void 0),
    receive_remote_changeset: vi.fn(),
    receive_resource_update: vi.fn(),
    current_heads: vi.fn(() => new Uint8Array()),
    attach_surface: vi.fn(),
    detach_surface: vi.fn(),
    resize_surface: vi.fn(),
    invalidate_surface: vi.fn(),
    refresh_surface: vi.fn(),
    render_surface: vi.fn<(_page: number, _revision: { value: number }) => { value: number } | undefined>(() => ({ value: 1 })),
    surface_backend: vi.fn(() => 'cpu'),
    page_external_elements: vi.fn(() => []),
    page_table_overlays: vi.fn(() => []),
    page_link_rects: vi.fn(() => []),
    table_overlays: vi.fn(() => []),
    link_rects: vi.fn(() => []),
    page_sizes: vi.fn<() => Size[]>(() => DefaultPageSizes),
    page_backing_sizes: vi.fn<() => Size[]>(() => DefaultPageSizes),
    selection_endpoints: vi.fn(() => void 0),
    selection: vi.fn(() => void 0),
    tracked_ranges: vi.fn<() => TrackedRange[]>(() => []),
    replace_viewport_anchor_presentation: vi.fn(() => true),
    root_attrs: vi.fn<() => PlainRootNode>(() => ({}) as PlainRootNode),
    cursor: vi.fn(() => void 0),
    root_modifiers: vi.fn(() => []),
    modifier_state: vi.fn(() => void 0),
    block_state: vi.fn(() => void 0),
    free: vi.fn(),
  };
}

async function createEditor(core = createCore(), installConsumer = true): Promise<{ editor: Editor; core: FakeCore }> {
  wasmHarness.createEditor.mockReturnValue(core);
  const editor = await Editor.createFromDoc({} as never, { width: 1, height: 1, scale_factor: 1 });
  if (installConsumer) installPublicationConsumer(editor);
  return { editor, core };
}

function presentActiveSurfaces(editor: Editor): void {
  untrack(() => {
    const requiredPages = editor.activeSurfacePages;
    editor.requestSurfacePages(requiredPages);
    const bundle = editor.publishIfReady(requiredPages);
    if (bundle && editor.acceptPublication(bundle)) editor.completePresentation(bundle);
  });
}

function installPublicationConsumer(editor: Editor): void {
  const activateVisualHost = editor.activateVisualHost.bind(editor);
  vi.spyOn(editor, 'activateVisualHost').mockImplementation(() => {
    const release = activateVisualHost();
    presentActiveSurfaces(editor);
    return release;
  });

  const attachSurface = editor.attachSurface.bind(editor);
  vi.spyOn(editor, 'attachSurface').mockImplementation((...args) => {
    const backend = attachSurface(...args);
    presentActiveSurfaces(editor);
    return backend;
  });

  const invalidateSurface = editor.invalidateSurface.bind(editor);
  vi.spyOn(editor, 'invalidateSurface').mockImplementation((page) => {
    invalidateSurface(page);
    presentActiveSurfaces(editor);
  });

  const recoverSurfaces = editor.recoverSurfaces.bind(editor);
  vi.spyOn(editor, 'recoverSurfaces').mockImplementation(() => {
    recoverSurfaces();
    presentActiveSurfaces(editor);
  });

  const resizeViewportNow = editor.resizeViewportNow.bind(editor);
  vi.spyOn(editor, 'resizeViewportNow').mockImplementation((width, height, scaleFactor) => {
    resizeViewportNow(width, height, scaleFactor);
    presentActiveSurfaces(editor);
  });
}

function enterTerminalState(editor: Editor, state: TerminalState): void {
  if (state === 'failed') editor.surfaceReplacementFailed(0);
  else editor.destroy();
}

function installScrollScope(editor: Editor): EditorScrollScope {
  const scope = new EditorScrollScope(editor, () => ({ enabled: false, position: undefined }));
  editor.registerScrollIntoView((options, request) => scope.scrollIntoView(options, request));
  return scope;
}

async function expectPending(promise: Promise<unknown> | undefined): Promise<void> {
  if (!promise) throw new Error('Expected a publication promise');
  let settled = false;
  void promise
    .then(() => {
      settled = true;
    })
    .catch(() => {
      settled = true;
    });
  await Promise.resolve();
  expect(settled).toBe(false);
}

describe('Editor guarded core invocation', () => {
  let frames: FrameRequestCallback[];

  beforeEach(() => {
    for (const editor of snapshot()) editor.destroy();
    frames = [];
    wasmHarness.createEditor.mockReset();
    wasmHarness.setThemeVariant.mockReset().mockReturnValue(null);
    sentryHarness.captureException.mockReset();
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.push((time) => {
        callback(time);
        for (const editor of snapshot()) presentActiveSurfaces(editor);
      });
      return frames.length;
    });
    vi.stubGlobal('cancelAnimationFrame', vi.fn());
  });

  it('discards a partially initialized core without registering a Host', async () => {
    const core = createCore();
    const error = new Error('initial admission failed');
    core.enqueue_request.mockImplementation(() => {
      throw error;
    });
    wasmHarness.createEditor.mockReturnValue(core);

    await expect(Editor.createFromDoc({} as never, { width: 1, height: 1, scale_factor: 1 })).rejects.toBe(error);

    expect(core.free).toHaveBeenCalledTimes(1);
    expect(snapshot()).toEqual([]);
  });

  it('reports the first terminal failure exactly once', async () => {
    const { editor } = await createEditor();
    const failure = new Error('terminal failure');

    editor.fail(failure);
    editor.fail(new Error('later failure'));

    expect(sentryHarness.captureException).toHaveBeenCalledOnce();
    expect(sentryHarness.captureException).toHaveBeenCalledWith(failure);
  });

  it('still contains the editor when terminal failure reporting throws', async () => {
    const { editor, core } = await createEditor();
    const failure = new Error('terminal failure');
    sentryHarness.captureException.mockImplementationOnce(() => {
      throw new Error('reporting failed');
    });

    editor.fail(failure);

    expect(editor.failure).toBe(failure);
    expect(core.free).toHaveBeenCalledOnce();
    expect(snapshot()).toEqual([]);
  });

  it('treats an initialize handler exception as pre-Host creation failure', async () => {
    const core = createCore();
    const error = new Error('initialize failed');
    core.tick_through.mockImplementation(() => {
      throw error;
    });
    wasmHarness.createEditor.mockReturnValue(core);

    await expect(Editor.createFromDoc({} as never, { width: 1, height: 1, scale_factor: 1 })).rejects.toBe(error);

    expect(core.free).toHaveBeenCalledTimes(1);
    expect(snapshot()).toEqual([]);
    expect(frames).toEqual([]);
  });

  it('retires a scheduled RAF when updateNow drains the queued request prefix', async () => {
    const { editor, core } = await createEditor();
    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    const scheduledFrame = frames.at(-1);

    expect(editor.hasQueuedTick).toBe(true);
    expect(scheduledFrame).toBeDefined();

    editor.updateNow((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));

    expect(editor.hasQueuedTick).toBe(false);
    expect(cancelAnimationFrame).toHaveBeenCalledWith(frames.length);
    scheduledFrame?.(0);
    expect(core.tick).not.toHaveBeenCalled();

    editor.destroy();
  });

  it('binds an admitted reveal before reconciling surfaces for the installed revision', async () => {
    const { editor, core } = await createEditor();
    const order: string[] = [];
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(0, document.createElement('canvas'), 100, 100, () => {
      order.push('replace');
    });
    editor.registerScrollIntoView((_options, request) => {
      request?.beforePublish(() => {
        order.push('before-publish');
      });
    });
    core.page_sizes.mockReturnValue([{ width: 100, height: 120 }, ...DefaultPageSizes.slice(1)]);
    core.page_backing_sizes.mockReturnValue([{ width: 100, height: 120 }, ...DefaultPageSizes.slice(1)]);

    editor.updateNow(() => {
      editor.enqueue({ type: 'history', op: { type: 'undo' } });
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });

    expect(order).toEqual(['before-publish', 'replace']);

    editor.registerScrollIntoView(null);
    releaseHost();
    editor.destroy();
  });

  it('rejects an admitted async update when its before-publish callback fails', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('before publish failed');
    const cleanupFailure = new Error('before publish cleanup failed');
    const consoleError = vi.spyOn(console, 'error').mockImplementation(vi.fn());
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [],
      request_outcomes: [{ request_id: { value: 1 }, command_outcomes: [{ type: 'applied' }] }],
    });

    const update = editor.update((request) => {
      request.enqueue({ type: 'history', op: { type: 'undo' } });
      request.beforePublish(
        () => {
          throw error;
        },
        () => {
          throw cleanupFailure;
        },
      );
    });

    frames.at(-1)?.(0);

    await expect(update).rejects.toBe(error);
    expect(editor.failure).toBe(error);
    expect(consoleError).toHaveBeenCalledWith('Editor request cleanup failed:', cleanupFailure);
  });

  it('rejects every request from a tick when a later before-publish callback fails', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('later before publish failed');
    core.enqueue_request.mockReturnValueOnce({ value: 1 }).mockReturnValueOnce({ value: 2 });
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [],
      request_outcomes: [
        { request_id: { value: 1 }, command_outcomes: [{ type: 'applied' }] },
        { request_id: { value: 2 }, command_outcomes: [{ type: 'applied' }] },
      ],
    });

    const first = editor.update((request) => {
      request.enqueue({ type: 'history', op: { type: 'undo' } });
    });
    const second = editor.update((request) => {
      request.enqueue({ type: 'history', op: { type: 'undo' } });
      request.beforePublish(() => {
        throw error;
      });
    });
    const assertions = Promise.all([expect(first).rejects.toBe(error), expect(second).rejects.toBe(error)]);

    frames.at(-1)?.(0);

    await assertions;
    expect(editor.failure).toBe(error);
  });

  it('rejects a terminal receipt even when its discard hook fails', async () => {
    const { editor } = await createEditor();
    const failure = new Error('terminal failure');
    const cleanupFailure = new Error('discard failed');
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {
      throw new Error('cleanup reporting failed');
    });
    const update = editor.update((request) => {
      request.enqueue({ type: 'history', op: { type: 'undo' } });
      request.beforePublish(vi.fn(), () => {
        throw cleanupFailure;
      });
    });
    const rejection = expect(update).rejects.toBe(failure);

    editor.fail(failure);

    await rejection;
    expect(editor.failure).toBe(failure);
    expect(consoleError).toHaveBeenCalledWith('Editor request cleanup failed:', cleanupFailure);
  });

  it('keeps an admitted receipt pending until surface reconciliation completes', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('surface replacement failed');
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(0, document.createElement('canvas'), 100, 100, () => {
      throw error;
    });
    core.page_sizes.mockReturnValue([{ width: 100, height: 120 }, ...DefaultPageSizes.slice(1)]);
    core.page_backing_sizes.mockReturnValue([{ width: 100, height: 120 }, ...DefaultPageSizes.slice(1)]);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }],
      request_outcomes: [{ request_id: { value: 1 }, command_outcomes: [{ type: 'applied' }] }],
    });

    const update = editor.update((request) => {
      request.enqueue({ type: 'history', op: { type: 'undo' } });
    });
    const rejection = expect(update).rejects.toBe(error);

    frames.at(-1)?.(0);

    await rejection;
    expect(editor.failure).toBe(error);
    releaseHost();
  });

  it('does not complete publication waiters until the accepted bundle is presented', async () => {
    const { editor } = await createEditor(createCore(), false);
    const releaseHost = editor.activateVisualHost();
    const requiredPages = new Set([0]);
    editor.requestSurfacePages(requiredPages);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    const bundle = editor.publishIfReady(requiredPages);
    if (!bundle) throw new Error('Expected a publishable bundle');
    const publication = editor.awaitPublishedRevision(bundle.snapshot.revision, { requireFrame: true });

    expect(editor.acceptPublication(bundle)).toBe(true);
    await expectPending(publication);

    editor.completePresentation(bundle);
    await expect(publication).resolves.toEqual({ type: 'published', revision: bundle.snapshot.revision });

    releaseHost();
    editor.destroy();
  });

  it.each([
    [
      'request admission',
      async (editor: Editor, core: FakeCore, error: Error) => {
        core.enqueue_request.mockImplementation(() => {
          throw error;
        });
        let thrown: unknown;
        try {
          editor.enqueue({ type: 'history', op: { type: 'undo' } });
        } catch (err) {
          thrown = err;
        }
        expect(thrown).toMatchObject({ message: 'core failed' });
      },
    ],
    [
      'remote admission',
      async (editor: Editor, core: FakeCore, error: Error) => {
        core.receive_remote_changeset.mockImplementation(() => {
          throw error;
        });
        await expect(editor.receiveRemoteChangeset(new Uint8Array([1]))).rejects.toMatchObject({ message: 'core failed' });
      },
    ],
    [
      'resource admission',
      async (editor: Editor, core: FakeCore, error: Error) => {
        core.receive_resource_update.mockImplementation(() => {
          throw error;
        });
        let thrown: unknown;
        try {
          editor.receiveResourceUpdate({} as never);
        } catch (err) {
          thrown = err;
        }
        expect(thrown).toMatchObject({ message: 'core failed' });
      },
    ],
    [
      'core read',
      async (editor: Editor, core: FakeCore, error: Error) => {
        core.current_heads.mockImplementation(() => {
          throw error;
        });
        expect(() => editor.currentHeads()).toThrowError(error);
      },
    ],
    [
      'surface core call',
      async (editor: Editor, core: FakeCore, error: Error) => {
        editor.activateVisualHost();
        core.attach_surface.mockImplementation(() => {
          throw error;
        });
        let thrown: unknown;
        try {
          editor.attachSurface(0, document.createElement('canvas'), 100, 100);
        } catch (err) {
          thrown = err;
        }
        expect(thrown).toMatchObject({ message: 'core failed' });
      },
    ],
  ])('turns an unexpected %s exception into one-shot Host failure', async (_name, invoke) => {
    const { editor, core } = await createEditor();
    const error = new Error('core failed');

    await invoke(editor, core, error);

    expect(editor.failure).toMatchObject({ message: 'core failed' });
    expect(core.free).toHaveBeenCalledTimes(1);
    expect(snapshot().includes(editor)).toBe(false);
  });

  it('fails on an unexpected normal tick exception', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('tick failed');
    core.tick.mockImplementation(() => {
      throw error;
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.failure).toBe(error);
    expect(core.free).toHaveBeenCalledTimes(1);
  });

  it('drains admitted work without waiting for animation frames while the document is hidden', async () => {
    const visibility = vi.spyOn(document, 'visibilityState', 'get').mockReturnValue('hidden');
    try {
      const { editor, core } = await createEditor();

      editor.enqueue({ type: 'history', op: { type: 'undo' } });
      expect(frames).toEqual([]);

      await Promise.resolve();

      expect(core.tick).toHaveBeenCalledTimes(1);
    } finally {
      visibility.mockRestore();
    }
  });

  it('fails when applied snapshot materialization throws', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('cursor read failed');
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['cursor'] }],
      request_outcomes: [],
    });
    core.cursor.mockImplementation(() => {
      throw error;
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.failure).toBe(error);
    expect(core.free).toHaveBeenCalledTimes(1);
  });

  it('treats an empty remote changeset as an already-applied no-op', async () => {
    const { editor, core } = await createEditor();

    await expect(editor.receiveRemoteChangeset(new Uint8Array())).resolves.toBe(editor.appliedRevision);

    expect(core.receive_remote_changeset).not.toHaveBeenCalled();
    expect(frames).toEqual([]);
  });

  it('turns exhausted surface replacement into one-shot Host failure', async () => {
    const { editor, core } = await createEditor();

    expect(editor.terminal).toBe(false);

    editor.surfaceReplacementFailed(3);
    editor.surfaceReplacementFailed(3);

    expect(editor.terminal).toBe(true);
    expect(editor.failure).toMatchObject({ message: 'Editor surface replacement failed for page 3' });
    expect(core.free).toHaveBeenCalledTimes(1);
    expect(snapshot().includes(editor)).toBe(false);
  });

  it('cancels a queued pointer-style refresh when the Editor becomes terminal', async () => {
    const { editor } = await createEditor();
    const refreshPointerStyle = vi.spyOn(editor, 'refreshPointerStyle');
    editor.updatePointerHover(10, 10);
    refreshPointerStyle.mockClear();

    editor.refreshPointerStyleAfterDomUpdate();
    editor.surfaceReplacementFailed(0);
    await tick();

    expect(refreshPointerStyle).not.toHaveBeenCalled();
  });

  it.each<TerminalState>(['failed', 'destroyed'])('returns null from update after the Editor is %s', async (state) => {
    const { editor, core } = await createEditor();
    enterTerminalState(editor, state);
    core.enqueue_request.mockClear();
    core.tick_through.mockClear();
    const build = vi.fn();

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    await expect(editor.update(build)).resolves.toBeNull();

    expect(editor.terminal).toBe(true);
    expect(build).not.toHaveBeenCalled();
    expect(core.enqueue_request).not.toHaveBeenCalled();
    expect(core.tick_through).not.toHaveBeenCalled();
  });

  it.each<TerminalState>(['failed', 'destroyed'])('returns null from updateNow after the Editor is %s', async (state) => {
    const { editor, core } = await createEditor();
    enterTerminalState(editor, state);
    core.enqueue_request.mockClear();
    core.tick_through.mockClear();
    const build = vi.fn();

    expect(editor.updateNow(build)).toBeNull();

    expect(editor.terminal).toBe(true);
    expect(build).not.toHaveBeenCalled();
    expect(core.enqueue_request).not.toHaveBeenCalled();
    expect(core.tick_through).not.toHaveBeenCalled();
  });

  it('preserves request-bound reveal admission when read-only filtering keeps the selection update', async () => {
    const { editor } = await createEditor();
    const scroll = installScrollScope(editor);
    editor.readOnly = true;

    const update = editor.updateNow(() => {
      editor.enqueue({ type: 'selection', op: { type: 'unset' } });
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });

    expect(update).not.toBeNull();
    expect(scroll.activateForRevision(update?.revision ?? -1)).toBe(scroll.pendingRequest);
  });

  it('discards an unbound reveal when read-only filtering rejects the whole update', async () => {
    const { editor } = await createEditor();
    const scroll = installScrollScope(editor);
    editor.readOnly = true;

    const update = editor.updateNow(() => {
      editor.enqueue({ type: 'history', op: { type: 'undo' } });
      editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });

    expect(update).toBeNull();
    expect(scroll.pendingRequest).toBeNull();
  });

  it('preserves an admitted updateNow failure before rejecting later admission', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('tick through failed');
    core.tick_through.mockImplementation(() => {
      throw error;
    });

    let thrown: unknown;
    try {
      editor.updateNow((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    } catch (err) {
      thrown = err;
    }

    expect(thrown).toBe(error);
    expect(editor.terminal).toBe(true);
    expect(editor.failure).toBe(error);

    core.enqueue_request.mockClear();
    core.tick_through.mockClear();
    const build = vi.fn();
    expect(editor.updateNow(build)).toBeNull();
    expect(build).not.toHaveBeenCalled();
    expect(core.enqueue_request).not.toHaveBeenCalled();
    expect(core.tick_through).not.toHaveBeenCalled();
  });

  it('preserves an admitted update failure before rejecting later admission', async () => {
    const { editor, core } = await createEditor();
    const error = new Error('tick failed');
    core.tick.mockImplementation(() => {
      throw error;
    });

    const pending = editor.update((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    frames.at(-1)?.(0);

    await expect(pending).rejects.toBe(error);
    expect(editor.terminal).toBe(true);
    expect(editor.failure).toBe(error);

    core.enqueue_request.mockClear();
    core.tick.mockClear();
    const build = vi.fn();
    await expect(editor.update(build)).resolves.toBeNull();
    expect(build).not.toHaveBeenCalled();
    expect(core.enqueue_request).not.toHaveBeenCalled();
    expect(core.tick).not.toHaveBeenCalled();
  });

  it('keeps updateNow reentrancy ahead of the terminal guard', async () => {
    const { editor } = await createEditor();

    expect(() =>
      editor.updateNow(() => {
        editor.destroy();
        editor.updateNow(() => {
          throw new Error('nested builder must not run');
        });
      }),
    ).toThrowError('updateNow cannot run reentrant-ly');
  });

  it('keeps expected target unavailability non-terminal', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.surface_backend.mockReturnValue('cpu-oversized');

    expect(editor.attachSurface(0, document.createElement('canvas'), 100, 100)).toBe('cpu-oversized');
    expect(editor.failure).toBeUndefined();

    const sameRevisionUpdate = editor.updateNow((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    const sameRevisionPublication = sameRevisionUpdate?.awaitPublished();
    await expect(sameRevisionPublication).resolves.toEqual({ type: 'published', revision: 1 });

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);

    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'render_invalidated' }],
      request_outcomes: [{ request_id: { value: 1 }, command_outcomes: [{ type: 'applied' }] }],
    });
    const pending = editor.update((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    frames.at(-1)?.(0);
    const update = await pending;

    await expect(update?.awaitPublished()).resolves.toEqual({ type: 'published', revision: 2 });
    expect(editor.failure).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('does not publish a missing surface target as a delivered frame', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.surface_backend.mockReturnValue('none');
    core.render_surface.mockClear();

    expect(editor.attachSurface(0, document.createElement('canvas'), 100, 100)).toBe('none');
    expect(core.render_surface).not.toHaveBeenCalled();
    expect(editor.publishedSurfaceCanvas(0)).toBeUndefined();

    const update = editor.updateNow((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    if (!update) throw new Error('Expected an editor update');
    const publication = editor.awaitPublishedRevision(update.revision, { requireFrame: true });
    await expectPending(publication);

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    await expect(publication).resolves.toEqual({ type: 'published', revision: 1 });
    expect(editor.failure).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('keeps the whole framed publication when the active host loses its last target', async () => {
    const { editor } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    const canvas = document.createElement('canvas');
    editor.attachSurface(0, canvas, 100, 100);
    const published = editor.published;

    editor.detachSurface(0);

    expect(editor.published).toBe(published);
    expect(editor.publishedSurfaceCanvas(0)).toBe(canvas);

    releaseHost();
    editor.destroy();
  });

  it('keeps an accepted publication valid while the producer cohort changes', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();

    try {
      editor.attachSurface(0, document.createElement('canvas'), 100, 100);

      core.surface_backend.mockReturnValue('none');
      editor.attachSurface(0, document.createElement('canvas'), 100, 100);
      const publication = editor.awaitPublishedRevision(1);
      const framedPublication = editor.awaitPublishedRevision(1, { requireFrame: true });

      await expect(publication).resolves.toEqual({ type: 'published', revision: 1 });
      await expect(framedPublication).resolves.toEqual({ type: 'published', revision: 1 });

      editor.detachSurface(0);
    } finally {
      releaseHost();
      editor.destroy();
    }
  });

  it('completes a request-bound reveal when its required surface revision fails', async () => {
    const { editor, core } = await createEditor(createCore(), false);
    const scroll = installScrollScope(editor);
    const releaseHost = editor.activateVisualHost((revision) => scroll.discardFailedForRevision(revision));
    const requiredPages = new Set([0]);
    core.render_surface.mockReset().mockReturnValueOnce({ value: 1 }).mockReturnValueOnce(undefined);
    editor.requestSurfacePages(requiredPages);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    const initial = editor.publishIfReady(requiredPages);
    if (!initial) throw new Error('Expected the initial surface publication');
    expect(editor.acceptPublication(initial)).toBe(true);
    editor.completePresentation(initial);
    core.tick_through.mockImplementation((requestId) => ({
      revision: { value: 2 },
      events: [{ type: 'render_invalidated' }],
      request_outcomes: [{ request_id: requestId, command_outcomes: [{ type: 'applied' }] }],
    }));

    let presentation: Promise<void> | undefined;
    editor.updateNow(() => {
      editor.enqueue({ type: 'history', op: { type: 'undo' } });
      presentation = editor.scrollIntoView({ target: { type: 'current_selection_head' }, policy: 'cursor_guard' });
    });
    let settled = false;
    void presentation?.then(() => {
      settled = true;
    });
    await Promise.resolve();

    expect(settled).toBe(true);
    expect(scroll.pendingRequest).toBeNull();

    releaseHost();
    editor.destroy();
  });

  it('rejects a failed render revision once and retries only after applied advances', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    const canvas = document.createElement('canvas');
    core.render_surface.mockReset().mockReturnValueOnce({ value: 1 }).mockReturnValueOnce(undefined).mockReturnValueOnce({ value: 3 });
    editor.attachSurface(0, canvas, 100, 100);
    core.tick
      .mockReturnValueOnce({
        revision: { value: 2 },
        events: [{ type: 'render_invalidated' }],
        request_outcomes: [],
      })
      .mockReturnValueOnce({
        revision: { value: 2 },
        events: [],
        request_outcomes: [],
      })
      .mockReturnValueOnce({
        revision: { value: 3 },
        events: [],
        request_outcomes: [],
      });
    let failedPublicationResult: unknown;
    let failedPublicationSettled = false;
    void editor
      .awaitPublishedRevision(2)
      .then((result) => {
        failedPublicationResult = result;
        failedPublicationSettled = true;
      })
      .catch((err: unknown) => {
        failedPublicationResult = err;
        failedPublicationSettled = true;
      });
    const laterPublication = editor.awaitPublishedRevision(3).catch((err: unknown) => err);

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    await Promise.resolve();
    await Promise.resolve();

    expect(failedPublicationSettled).toBe(true);
    expect(failedPublicationResult).toMatchObject({ name: 'OperationError' });
    expect(editor.publishedRevision).toBe(1);
    expect(editor.publishedSurfaceCanvas(0)).toBe(canvas);
    expect(core.render_surface.mock.calls.map(([, revision]) => revision.value)).toEqual([1, 2]);
    let lateFailedPublication: unknown;
    void editor.awaitPublishedRevision(2).catch((err: unknown) => {
      lateFailedPublication = err;
    });
    await Promise.resolve();
    expect(lateFailedPublication).toMatchObject({ name: 'OperationError' });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    expect(core.render_surface.mock.calls.map(([, revision]) => revision.value)).toEqual([1, 2]);

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    await expect(laterPublication).resolves.toEqual({ type: 'published', revision: 3 });
    expect(core.render_surface.mock.calls.map(([, revision]) => revision.value)).toEqual([1, 2, 3]);
    expect(editor.failure).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('returns an already matching publication before a newer failed revision', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.render_surface.mockReset().mockReturnValueOnce({ value: 1 }).mockReturnValueOnce(undefined);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'render_invalidated' }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    await expect(editor.awaitPublishedRevision(1)).resolves.toEqual({ type: 'published', revision: 1 });

    releaseHost();
    editor.destroy();
  });

  it('optionally waits for the first framed publication after the zero-target bootstrap', async () => {
    const { editor } = await createEditor();
    const releaseHost = editor.activateVisualHost();

    await expect(editor.awaitPublishedRevision(1)).resolves.toEqual({ type: 'published', revision: 1 });

    const framedPublication = editor.awaitPublishedRevision(1, { requireFrame: true });
    let framedPublicationSettled = false;
    void framedPublication.finally(() => {
      framedPublicationSettled = true;
    });
    await Promise.resolve();
    expect(framedPublicationSettled).toBe(false);

    const canvas = document.createElement('canvas');
    editor.attachSurface(0, canvas, 100, 100);

    await expect(framedPublication).resolves.toEqual({ type: 'published', revision: 1 });
    expect(editor.publishedSurfaceCanvas(0)).toBe(canvas);

    releaseHost();
    editor.destroy();
  });

  it('rejects a frame-required bootstrap waiter when its same-revision render fails', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.render_surface.mockReturnValue(undefined);

    const framedPublication = editor.awaitPublishedRevision(1, { requireFrame: true });
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);

    await expect(framedPublication).rejects.toMatchObject({ name: 'OperationError' });
    expect(editor.publishedRevision).toBe(1);
    expect(editor.publishedSurfaceCanvas(0)).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('keeps no-host semantics for a frame-required publication waiter', async () => {
    const { editor } = await createEditor();

    await expect(editor.awaitPublishedRevision(1, { requireFrame: true })).resolves.toEqual({ type: 'no_host' });

    editor.destroy();
  });

  it('retries a failed revision after a real surface key change', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.render_surface.mockReset().mockReturnValueOnce({ value: 1 }).mockReturnValueOnce(undefined);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'render_invalidated' }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    core.render_surface.mockReturnValue({ value: 2 });

    editor.invalidateSurface(0);

    expect(core.render_surface.mock.calls.map(([, revision]) => revision.value)).toEqual([1, 2, 2]);
    await expect(editor.awaitPublishedRevision(2)).resolves.toEqual({ type: 'published', revision: 2 });

    releaseHost();
    editor.destroy();
  });

  it('returns the accepted publication when registered after a target becomes unavailable', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);

    const waiterAtFailure = editor.awaitPublishedRevision(2);
    core.surface_backend.mockReturnValue('cpu-oversized');
    editor.invalidateSurface(0);

    await expect(waiterAtFailure).rejects.toMatchObject({ name: 'OperationError' });

    const replacementPublication = editor.awaitPublishedRevision(1);
    await expect(replacementPublication).resolves.toEqual({ type: 'published', revision: 1 });

    core.surface_backend.mockReturnValue('cpu');
    core.render_surface.mockReturnValue({ value: 2 });
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);

    releaseHost();
    editor.destroy();
  });

  it('pulls toolbar state once after a suspended selection drag reaches publication', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    const initialModifierState = { marker: 'initial-modifier' };
    const initialBlockState = { marker: 'initial-block' };
    const finalModifierState = { marker: 'final-modifier' };
    const finalBlockState = { marker: 'final-block' };
    core.modifier_state.mockReturnValue(initialModifierState as never);
    core.block_state.mockReturnValue(initialBlockState as never);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    core.tick.mockReturnValueOnce({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['modifiers', 'block'] }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.modifierState).toBe(initialModifierState);
    expect(editor.blockState).toBe(initialBlockState);
    core.modifier_state.mockClear().mockReturnValue(finalModifierState as never);
    core.block_state.mockClear().mockReturnValue(finalBlockState as never);
    core.render_surface.mockReturnValue(undefined);
    core.tick
      .mockReturnValueOnce({
        revision: { value: 3 },
        events: [{ type: 'state_changed', fields: ['modifiers', 'block'] }, { type: 'render_invalidated' }],
        request_outcomes: [],
      })
      .mockReturnValueOnce({
        revision: { value: 3 },
        events: [{ type: 'state_changed', fields: ['modifiers', 'block'] }],
        request_outcomes: [],
      });

    editor.suspendToolbarSync();
    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(core.modifier_state).not.toHaveBeenCalled();
    expect(core.block_state).not.toHaveBeenCalled();
    expect(editor.modifierState).toBe(initialModifierState);
    expect(editor.blockState).toBe(initialBlockState);

    editor.resumeToolbarSync();

    expect(core.modifier_state).not.toHaveBeenCalled();
    expect(core.block_state).not.toHaveBeenCalled();
    expect(editor.publishedRevision).toBe(2);

    core.render_surface.mockReturnValue({ value: 3 });
    editor.invalidateSurface(0);
    editor.resumeToolbarSync();
    editor.invalidateSurface(0);

    expect(core.modifier_state).toHaveBeenCalledTimes(1);
    expect(core.block_state).toHaveBeenCalledTimes(1);
    expect(editor.modifierState).toBe(finalModifierState);
    expect(editor.blockState).toBe(finalBlockState);
    expect(editor.publishedRevision).toBe(3);

    releaseHost();
    editor.destroy();
  });

  it('keeps late toolbar resume inert after Host failure', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    editor.suspendToolbarSync();
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['modifiers', 'block'] }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.publishedRevision).toBe(2);
    expect(core.modifier_state).not.toHaveBeenCalled();
    expect(core.block_state).not.toHaveBeenCalled();

    editor.surfaceReplacementFailed(0);

    expect(() => editor.resumeToolbarSync()).not.toThrow();
    expect(core.modifier_state).not.toHaveBeenCalled();
    expect(core.block_state).not.toHaveBeenCalled();

    releaseHost();
    editor.destroy();
  });

  it('keeps an oversized target unavailable after invalidation', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    core.surface_backend.mockReturnValue('cpu-oversized');

    editor.invalidateSurface(0);
    const update = editor.updateNow((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    const publication = update?.awaitPublished();
    await expect(publication).resolves.toEqual({ type: 'published', revision: 1 });

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    expect(editor.failure).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('keeps an oversized target unavailable after surface recovery', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    core.surface_backend.mockReturnValue('cpu-oversized');

    editor.recoverSurfaces();
    const update = editor.updateNow((request) => request.enqueue({ type: 'history', op: { type: 'undo' } }));
    const publication = update?.awaitPublished();
    await expect(publication).resolves.toEqual({ type: 'published', revision: 1 });

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    expect(editor.failure).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('drops context menu contributors when destroyed', async () => {
    const { editor } = await createEditor();
    editor.registerContextMenuContributor(() => [{ label: 'item', onclick: vi.fn() }]);

    editor.destroy();

    expect(editor.collectContextMenuContributions({ hit: undefined, clientX: 0, clientY: 0 })).toEqual([]);
  });

  it('keeps the published target intact when a changed surface configuration has no replacement', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    const canvas = document.createElement('canvas');
    core.render_surface.mockReturnValueOnce({ value: 1 }).mockReturnValue(undefined);
    editor.attachSurface(0, canvas, 100, 100);

    core.page_sizes.mockReturnValue([{ width: 200, height: 300 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 200, height: 400 }]);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }, { type: 'render_invalidated' }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(core.resize_surface).not.toHaveBeenCalled();
    expect(core.render_surface).toHaveBeenCalledTimes(1);
    expect(editor.appliedRevision).toBe(2);
    expect(editor.publishedRevision).toBe(1);
    expect(editor.pageSizes).toEqual(DefaultPageSizes);
    expect(editor.publishedSurfaceCanvas(0)).toBe(canvas);

    releaseHost();
    editor.destroy();
  });

  it('replaces a changed surface configuration without clearing the published target in place', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    const displayed = document.createElement('canvas');
    const candidate = document.createElement('canvas');
    core.render_surface.mockReset().mockReturnValueOnce({ value: 1 }).mockReturnValue(undefined);
    const replace = vi.fn(() => {
      editor.attachSurface(0, candidate, 200, 400, replace);
    });
    editor.attachSurface(0, displayed, 100, 100, replace);

    core.page_sizes.mockReturnValue([{ width: 200, height: 300 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 200, height: 400 }]);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }, { type: 'render_invalidated' }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(replace).toHaveBeenCalledTimes(1);
    expect(core.resize_surface).not.toHaveBeenCalled();
    expect(core.detach_surface.mock.invocationCallOrder[0]).toBeLessThan(core.attach_surface.mock.invocationCallOrder[1]);
    expect(editor.publishedRevision).toBe(1);
    expect(editor.publishedSurfaceCanvas(0)).toBe(displayed);

    core.render_surface.mockReturnValue({ value: 2 });
    editor.invalidateSurface(0);

    expect(editor.publishedRevision).toBe(2);
    expect(editor.publishedSurfaceCanvas(0)).toBe(candidate);

    releaseHost();
    editor.destroy();
  });

  it('publishes the immediately applied viewport only after its replacement frame', async () => {
    const core = createCore();
    core.page_sizes.mockReturnValue([{ width: 40, height: 1064 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 40, height: 1064 }]);
    core.tick_through.mockImplementation((requestId) => ({
      revision: { value: 1 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }],
      request_outcomes: [{ request_id: requestId, command_outcomes: [{ type: 'applied' }] }],
    }));
    const { editor } = await createEditor(core);
    const releaseHost = editor.activateVisualHost();
    const displayed = document.createElement('canvas');
    const candidate = document.createElement('canvas');
    core.render_surface.mockReset().mockReturnValueOnce({ value: 1 }).mockReturnValue(undefined);
    const replace = vi.fn(() => {
      editor.attachSurface(0, candidate, 320, 1064, replace);
    });
    editor.attachSurface(0, displayed, 40, 1064, replace);

    core.page_sizes.mockReturnValue([{ width: 320, height: 1064 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 320, height: 1064 }]);
    core.tick_through.mockImplementation((requestId) => ({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }, { type: 'render_invalidated' }],
      request_outcomes: [{ request_id: requestId, command_outcomes: [{ type: 'applied' }] }],
    }));

    editor.resizeViewportNow(320, 800, 1);
    expect(editor.appliedRevision).toBe(2);
    expect(editor.publishedRevision).toBe(1);
    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(false);

    core.render_surface.mockReturnValue({ value: 2 });
    editor.invalidateSurface(0);

    expect(editor.publishedRevision).toBe(2);
    expect(editor.publishedSurfaceCanvas(0)).toBe(candidate);
    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true);

    releaseHost();
    editor.destroy();
  });

  it('publishes an already matching viewport when its first frame arrives', async () => {
    const core = createCore();
    core.page_sizes.mockReturnValue([{ width: 320, height: 1064 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 320, height: 1064 }]);
    core.tick_through.mockImplementation((requestId) => ({
      revision: { value: 1 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }],
      request_outcomes: [{ request_id: requestId, command_outcomes: [{ type: 'applied' }] }],
    }));
    wasmHarness.createEditor.mockReturnValue(core);
    const editor = await Editor.createFromDoc({} as never, { width: 320, height: 800, scale_factor: 1 });
    installPublicationConsumer(editor);
    const releaseHost = editor.activateVisualHost();

    editor.resizeViewportNow(320, 800, 1);
    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(false);

    editor.attachSurface(0, document.createElement('canvas'), 320, 1064);

    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true);

    releaseHost();
    editor.destroy();
  });

  it('requires a frame from the current visual host after reactivation', async () => {
    const core = createCore();
    core.page_sizes.mockReturnValue([{ width: 320, height: 1064 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 320, height: 1064 }]);
    core.tick_through.mockImplementation((requestId) => ({
      revision: { value: 1 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }],
      request_outcomes: [{ request_id: requestId, command_outcomes: [{ type: 'applied' }] }],
    }));
    wasmHarness.createEditor.mockReturnValue(core);
    const editor = await Editor.createFromDoc({} as never, { width: 320, height: 800, scale_factor: 1 });
    installPublicationConsumer(editor);
    const releaseFirstHost = editor.activateVisualHost();

    editor.resizeViewportNow(320, 800, 1);
    editor.attachSurface(0, document.createElement('canvas'), 320, 1064);
    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true);

    releaseFirstHost();
    const releaseSecondHost = editor.activateVisualHost();

    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(false);

    const secondCanvas = document.createElement('canvas');
    editor.attachSurface(0, secondCanvas, 320, 1064);

    expect(editor.publishedSurfaceCanvas(0)).toBe(secondCanvas);
    expect(editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true);

    releaseSecondHost();
    editor.destroy();
  });

  it('accepts an empty publication for the current visual host after reactivation', async () => {
    const { editor } = await createEditor();
    const releaseFirstHost = editor.activateVisualHost();

    expect(editor.isPublished(editor.appliedRevision)).toBe(true);

    releaseFirstHost();
    const releaseSecondHost = editor.activateVisualHost();

    expect(editor.isPublished(editor.appliedRevision)).toBe(true);

    releaseSecondHost();
    editor.destroy();
  });

  it('materializes table and link geometry only for active page targets', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.page_table_overlays.mockReturnValue([{ table_id: 'table-1' }] as never);
    core.page_link_rects.mockReturnValue([{ href: 'https://example.com' }] as never);
    editor.attachSurface(2, document.createElement('canvas'), 100, 100);
    core.page_table_overlays.mockClear();
    core.page_link_rects.mockClear();
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['table_overlays', 'link_rects'] }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(core.page_table_overlays).toHaveBeenCalledExactlyOnceWith(2);
    expect(core.page_link_rects).toHaveBeenCalledExactlyOnceWith(2);
    expect(core.table_overlays).not.toHaveBeenCalled();
    expect(core.link_rects).not.toHaveBeenCalled();
    expect(editor.tableOverlays).toEqual([{ table_id: 'table-1' }]);
    expect(editor.linkRects).toEqual([{ href: 'https://example.com' }]);

    releaseHost();
    editor.destroy();
  });

  it('keeps continuous table overlays document-scoped so a split table is not repeated per page', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.root_attrs.mockReturnValue({ layout_mode: { type: 'continuous', max_width: 800 } } as PlainRootNode);
    core.page_table_overlays.mockReturnValue([{ table_id: 'table-1' }] as never);
    core.table_overlays.mockReturnValue([{ table_id: 'table-1', page_idx: 0 }] as never);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    editor.attachSurface(1, document.createElement('canvas'), 100, 100);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['root_attrs', 'table_overlays'] }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.tableOverlays).toEqual([{ table_id: 'table-1', page_idx: 0 }]);
    expect(new Set(editor.tableOverlays.map((overlay) => overlay.table_id)).size).toBe(editor.tableOverlays.length);

    releaseHost();
    editor.destroy();
  });

  it('advances the applied selection signal only for selection changes', async () => {
    const { editor, core } = await createEditor();
    core.tick
      .mockReturnValueOnce({
        revision: { value: 2 },
        events: [{ type: 'state_changed', fields: ['doc'] }],
        request_outcomes: [],
      })
      .mockReturnValueOnce({
        revision: { value: 3 },
        events: [{ type: 'state_changed', fields: ['selection'] }],
        request_outcomes: [],
      });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    expect(editor.appliedSelectionRevision).toBe(0);

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    expect(editor.appliedSelectionRevision).toBe(1);

    editor.destroy();
  });

  it('advances the applied IME signal only for IME changes', async () => {
    const { editor, core } = await createEditor();
    core.tick
      .mockReturnValueOnce({
        revision: { value: 2 },
        events: [{ type: 'state_changed', fields: ['table_overlays'] }],
        request_outcomes: [],
      })
      .mockReturnValueOnce({
        revision: { value: 3 },
        events: [{ type: 'state_changed', fields: ['ime'] }],
        request_outcomes: [],
      });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    expect(editor.appliedImeRevision).toBe(0);

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    expect(editor.appliedImeRevision).toBe(1);

    editor.destroy();
  });

  it('reads tracked selection data from applied state while publication is pending', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    core.render_surface.mockReturnValueOnce({ value: 1 }).mockReturnValue(undefined);
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    const range = {
      id: 'spellcheck-1',
      anchor: { node: 'text', offset: 1, affinity: 'downstream' },
      head: { node: 'text', offset: 4, affinity: 'downstream' },
      rects: [{ page_idx: 0, rect: { x: 10, y: 20, width: 30, height: 10 } }],
    } as TrackedRange;
    core.tracked_ranges.mockReturnValue([range]);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['tracked_ranges'] }, { type: 'render_invalidated' }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.appliedRevision).toBe(2);
    expect(editor.publishedRevision).toBe(1);
    expect(editor.appliedSnapshot.trackedRanges).toEqual([range]);
    expect(editor.published?.snapshot.trackedRanges).toEqual([]);

    releaseHost();
    editor.destroy();
  });

  it('does not make visual-host activation depend on the publication it installs', async () => {
    const { editor } = await createEditor();
    const tracked = createTrackedEffect(() => editor.activateVisualHost());

    await tracked.flush();
    await tracked.flush();

    expect(tracked.runs()).toBe(1);

    tracked.destroy();
    editor.destroy();
  });

  it('does not make surface attachment depend on the applied or published snapshot it replaces', async () => {
    const { editor } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    const canvas = document.createElement('canvas');
    const tracked = createTrackedEffect(() => {
      editor.attachSurface(0, canvas, 100, 100);
      return;
    });

    await tracked.flush();
    await tracked.flush();

    expect(tracked.runs()).toBe(1);

    tracked.destroy();
    releaseHost();
    editor.destroy();
  });
});
