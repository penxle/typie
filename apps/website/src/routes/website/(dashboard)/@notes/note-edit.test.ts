import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NoteEdits } from '$lib/note/note-edit-state.svelte';
import NoteEditTestHost from './note-edit-test-host.svelte';

type NoteEditsOptions = ConstructorParameters<typeof NoteEdits>[0];
type NoteSaveOutcome = Awaited<ReturnType<NoteEditsOptions['save']>>;

describe('global note editing', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe() {
          return null;
        }
        disconnect() {
          return null;
        }
      },
    );
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('keeps saved content and color drafts while the query still has the previous snapshot', async () => {
    const save = vi.fn(async ({ field, value }: { field: 'content' | 'color'; value: string }) => ({
      kind: 'saved' as const,
      snapshot: {
        content: field === 'content' ? value : 'server content',
        color: field === 'color' ? value : 'gray',
      },
    }));
    const noteEdits = new NoteEdits({
      isTerminallyDeleted: () => false,
      save: async ({ field, value }) => save({ field, value }),
    });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteEditTestHost, { target, props: { noteEdits } });

    try {
      await tick();
      const textarea = target.querySelector('textarea');
      const blue = target.querySelector<HTMLButtonElement>('[data-note-color-value="blue"]');
      expect(textarea).not.toBeNull();
      expect(blue).not.toBeNull();
      if (!textarea || !blue) return;

      textarea.value = 'edited content';
      textarea.dispatchEvent(new InputEvent('input', { bubbles: true }));
      blue.click();
      await tick();

      expect(textarea.value).toBe('edited content');
      expect(blue.getAttribute('aria-pressed')).toBe('true');

      await vi.advanceTimersByTimeAsync(300);
      await tick();

      expect(save.mock.calls).toEqual([[{ field: 'color', value: 'blue' }], [{ field: 'content', value: 'edited content' }]]);
      expect(textarea.value).toBe('edited content');
      expect(blue.getAttribute('aria-pressed')).toBe('true');
    } finally {
      await unmount(component);
      target.remove();
    }
  });

  it('keeps a reserved save-status region at the end of the expanded toolbar', async () => {
    const pendingSave = Promise.withResolvers<NoteSaveOutcome>();
    const noteEdits = new NoteEdits({
      isTerminallyDeleted: () => false,
      save: () => pendingSave.promise,
    });
    const target = document.createElement('div');
    document.body.append(target);
    const component = mount(NoteEditTestHost, { target, props: { noteEdits } });

    try {
      await tick();
      const textarea = target.querySelector('textarea');
      const addEntityButton = [...target.querySelectorAll('button')].find((button) => button.textContent?.trim() === '연결 추가');
      const toolbar = addEntityButton?.parentElement;
      const statusRegion = toolbar?.querySelector('[aria-live="polite"]');
      expect(textarea).not.toBeNull();
      expect(toolbar).not.toBeNull();
      expect(statusRegion).not.toBeNull();
      if (!textarea || !statusRegion) return;

      expect(statusRegion.textContent).toBe('');

      textarea.value = 'edited content';
      textarea.dispatchEvent(new InputEvent('input', { bubbles: true }));
      await vi.advanceTimersByTimeAsync(800);
      await tick();

      expect(statusRegion.textContent).toBe('저장 중...');

      pendingSave.resolve({ kind: 'failed' });
      await Promise.resolve();
      await Promise.resolve();
      await tick();

      expect(statusRegion.textContent?.trim()).toBe('저장 실패');
      expect(statusRegion.querySelector('button')).toBeNull();
    } finally {
      await unmount(component);
      target.remove();
    }
  });
});
