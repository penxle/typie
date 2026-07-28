import { mount, tick, unmount } from 'svelte';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { Editor } from './editor.svelte';
import { createTrackedEffect } from './editor-effect-harness.svelte';
import PageLifecycleTestHost from './page-lifecycle-test-host.svelte';
import { snapshot } from './registry';
import type { Size, TickResult, TrackedRange } from '@typie/editor-ffi/browser';

const wasmHarness = vi.hoisted(() => ({
  createEditor: vi.fn(),
  setThemeVariant: vi.fn(() => null),
}));

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

function createCore() {
  return {
    enqueue_request: vi.fn(() => ({ value: 1 })),
    tick_through: vi.fn<(requestId: { value: number }) => TickResult>((requestId) => ({
      revision: { value: 1 },
      events: [],
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
    page_sizes: vi.fn<() => Size[]>(() => []),
    page_backing_sizes: vi.fn<() => Size[]>(() => []),
    selection_endpoints: vi.fn(() => void 0),
    selection: vi.fn(() => void 0),
    tracked_ranges: vi.fn<() => TrackedRange[]>(() => []),
    cursor: vi.fn(() => void 0),
    root_modifiers: vi.fn(() => []),
    modifier_state: vi.fn(() => void 0),
    block_state: vi.fn(() => void 0),
    free: vi.fn(),
  };
}

async function createEditor(core = createCore()): Promise<{ editor: Editor; core: FakeCore }> {
  wasmHarness.createEditor.mockReturnValue(core);
  const editor = await Editor.createFromDoc({} as never, { width: 1, height: 1, scale_factor: 1 });
  return { editor, core };
}

function enterTerminalState(editor: Editor, state: TerminalState): void {
  if (state === 'failed') editor.surfaceReplacementFailed(0);
  else editor.destroy();
}

async function expectPending(promise: Promise<unknown> | undefined): Promise<void> {
  if (!promise) throw new Error('Expected a publication promise');
  let settled = false;
  void promise.then(
    () => {
      settled = true;
    },
    () => {
      settled = true;
    },
  );
  await Promise.resolve();
  expect(settled).toBe(false);
}

class OffscreenIntersectionObserver {
  private readonly callback: IntersectionObserverCallback;

  constructor(callback: IntersectionObserverCallback) {
    this.callback = callback;
  }

  observe(target: Element): void {
    this.callback([{ isIntersecting: false, target } as IntersectionObserverEntry], this as never);
  }

  unobserve(): void {
    // Each test observer delivers its only entry synchronously.
  }

  disconnect(): void {
    // Each test observer delivers its only entry synchronously.
  }

  takeRecords(): IntersectionObserverEntry[] {
    return [];
  }
}

class PassiveResizeObserver {
  observe(): void {
    // Root geometry is fixed for this lifecycle test.
  }

  unobserve(): void {
    // Root geometry is fixed for this lifecycle test.
  }

  disconnect(): void {
    // Root geometry is fixed for this lifecycle test.
  }
}

describe('Editor guarded core invocation', () => {
  let frames: FrameRequestCallback[];

  beforeEach(() => {
    for (const editor of snapshot()) editor.destroy();
    frames = [];
    wasmHarness.createEditor.mockReset();
    wasmHarness.setThemeVariant.mockReset().mockReturnValue(null);
    vi.stubGlobal('requestAnimationFrame', (callback: FrameRequestCallback) => {
      frames.push(callback);
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
    await expectPending(sameRevisionPublication);

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    await expect(sameRevisionPublication).resolves.toEqual({ type: 'published', revision: 1 });

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
    const publication = update?.awaitPublished();
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
    void editor.awaitPublishedRevision(2).then(
      (result) => {
        failedPublicationResult = result;
        failedPublicationSettled = true;
      },
      (err: unknown) => {
        failedPublicationResult = err;
        failedPublicationSettled = true;
      },
    );
    const laterPublication = editor.awaitPublishedRevision(3).catch((err: unknown) => err);

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
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

  it('waits for a replacement publication when registered after a target becomes unavailable', async () => {
    const { editor, core } = await createEditor();
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);

    const waiterAtFailure = editor.awaitPublishedRevision(2);
    core.surface_backend.mockReturnValue('cpu-oversized');
    editor.invalidateSurface(0);

    await expect(waiterAtFailure).rejects.toMatchObject({ name: 'OperationError' });

    const replacementPublication = editor.awaitPublishedRevision(1);
    await expectPending(replacementPublication);

    core.surface_backend.mockReturnValue('cpu');
    core.render_surface.mockReturnValue({ value: 2 });
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);

    await expect(replacementPublication).resolves.toEqual({ type: 'published', revision: 1 });

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
    await expectPending(publication);

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    await expect(publication).resolves.toEqual({ type: 'published', revision: 1 });
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
    await expectPending(publication);

    core.surface_backend.mockReturnValue('cpu');
    editor.attachSurface(0, document.createElement('canvas'), 100, 100);
    await expect(publication).resolves.toEqual({ type: 'published', revision: 1 });
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
    expect(editor.pageSizes).toEqual([]);
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

  it('requests a preparing surface when page shrink removes every active target', async () => {
    const core = createCore();
    core.page_sizes.mockReturnValue([
      { width: 100, height: 100 },
      { width: 100, height: 100 },
    ]);
    core.page_backing_sizes.mockReturnValue([
      { width: 100, height: 100 },
      { width: 100, height: 100 },
    ]);
    const { editor } = await createEditor(core);
    const releaseHost = editor.activateVisualHost();
    editor.attachSurface(1, document.createElement('canvas'), 100, 100);
    expect(editor.publishedRevision).toBe(1);

    core.page_sizes.mockReturnValue([{ width: 200, height: 300 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 200, height: 400 }]);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }, { type: 'render_invalidated' }],
      request_outcomes: [],
    });

    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);

    expect(editor.appliedRevision).toBe(2);
    expect(editor.publishedRevision).toBe(1);
    expect(editor.preparingPage).toBe(0);

    editor.attachSurface(0, document.createElement('canvas'), 200, 400);

    expect(editor.publishedRevision).toBe(2);
    expect(editor.preparingPage).toBeUndefined();

    releaseHost();
    editor.destroy();
  });

  it('keeps page preparation latched until proof and cancels it on terminal failure without dropping the published frame', async () => {
    const core = createCore();
    core.page_sizes.mockReturnValue([{ width: 100, height: 100 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 100, height: 100 }]);
    const { editor } = await createEditor(core);
    const releaseHost = editor.activateVisualHost();
    const publishedCanvas = document.createElement('canvas');
    editor.attachSurface(0, publishedCanvas, 100, 100);

    editor.detachSurface(0);
    core.page_sizes.mockReturnValue([{ width: 200, height: 300 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 200, height: 400 }]);
    core.tick.mockReturnValue({
      revision: { value: 2 },
      events: [{ type: 'state_changed', fields: ['page_sizes'] }, { type: 'render_invalidated' }],
      request_outcomes: [],
    });
    editor.enqueue({ type: 'history', op: { type: 'undo' } });
    frames.at(-1)?.(0);
    expect(editor.preparingPage).toBe(0);

    core.render_surface.mockReturnValueOnce(undefined);
    const preparingCanvas = document.createElement('canvas');
    editor.attachSurface(0, preparingCanvas, 100, 100);
    expect(core.attach_surface).toHaveBeenLastCalledWith(0, preparingCanvas, 200, 400, 1);
    expect(editor.preparingPage).toBe(0);
    expect(editor.publishedSurfaceCanvas(0)).toBe(publishedCanvas);

    editor.surfaceReplacementFailed(0);
    expect(editor.preparingPage).toBeUndefined();
    expect(editor.publishedSurfaceCanvas(0)).toBe(publishedCanvas);

    releaseHost();
    editor.destroy();
  });

  it('activates an offscreen page while the Host is preparing its replacement surface', async () => {
    const core = createCore();
    core.page_sizes.mockReturnValue([{ width: 100, height: 100 }]);
    core.page_backing_sizes.mockReturnValue([{ width: 100, height: 100 }]);
    const { editor } = await createEditor(core);
    const releaseHost = editor.activateVisualHost();
    const initialCanvas = document.createElement('canvas');
    editor.attachSurface(0, initialCanvas, 100, 100);
    editor.detachSurface(0);
    core.attach_surface.mockClear();
    core.detach_surface.mockClear();
    core.render_surface.mockReset().mockReturnValue(undefined);

    vi.stubGlobal('IntersectionObserver', OffscreenIntersectionObserver);
    vi.stubGlobal('ResizeObserver', PassiveResizeObserver);
    const rectSpy = vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
      return this.dataset.pageLifecycleScrollRoot === undefined ? new DOMRect(0, 300, 200, 100) : new DOMRect(0, 0, 100, 100);
    });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(PageLifecycleTestHost, { target, props: { editor } });

    try {
      await tick();
      expect(core.attach_surface).not.toHaveBeenCalled();

      core.page_sizes.mockReturnValue([{ width: 200, height: 300 }]);
      core.page_backing_sizes.mockReturnValue([{ width: 200, height: 400 }]);
      core.tick.mockReturnValue({
        revision: { value: 2 },
        events: [{ type: 'state_changed', fields: ['page_sizes'] }, { type: 'render_invalidated' }],
        request_outcomes: [],
      });
      editor.enqueue({ type: 'history', op: { type: 'undo' } });
      frames.at(-1)?.(0);
      await tick();

      expect(editor.preparingPage).toBe(0);
      expect(core.attach_surface).toHaveBeenCalledExactlyOnceWith(0, expect.any(HTMLCanvasElement), 200, 400, 1);

      core.render_surface.mockReturnValue({ value: 2 });
      editor.invalidateSurface(0);
      await tick();

      expect(editor.publishedRevision).toBe(2);
      expect(editor.preparingPage).toBeUndefined();
      expect(core.detach_surface).toHaveBeenCalledExactlyOnceWith(0);
    } finally {
      await unmount(component);
      target.remove();
      rectSpy.mockRestore();
      releaseHost();
      editor.destroy();
      vi.unstubAllGlobals();
    }
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
