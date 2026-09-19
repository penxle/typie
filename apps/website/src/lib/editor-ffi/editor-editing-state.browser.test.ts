import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Editor } from './editor.svelte';
import EditorFrameSyncTestHost from './editor-frame-sync-test-host.svelte';
import type { PlainDoc } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const doc: PlainDoc = {
  root: {
    node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
    modifiers: {} as never,
    carry: [],
    children: [{ node: { type: 'paragraph' }, modifiers: {} as never, carry: [], children: [] }],
  },
};

describe('Editor accepted editing work', () => {
  let editor: Editor;
  let mounted: Record<string, unknown> | undefined;
  let target: HTMLElement | undefined;
  afterEach(async () => {
    if (mounted) await unmount(mounted);
    mounted = undefined;
    target?.remove();
    editor?.destroy();
    vi.unstubAllGlobals();
  });

  const create = async () => {
    editor = await Editor.createFromDoc(doc, { width: 320, height: 180, scale_factor: 1 });
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
  };
  const insert = (text: string) => editor.enqueue({ type: 'insertion', op: { type: 'text', text } });

  it('reports queued local work synchronously and applies it without waiting for an animation frame', async () => {
    await create();
    const pending: boolean[] = [];
    editor.localEdits.onChange(() => {
      pending.push(editor.localEdits.pending);
    });

    insert('preserved');
    expect(pending).toContain(true);
    expect(editor.localEdits.pending).toBe(true);
    editor.settlePendingEdits();
    expect(editor.localEdits.pending).toBe(false);
    expect(editor.proseText()).toContain('preserved');
  });

  it('one released stop cannot reopen mutation admission held by another caller', async () => {
    await create();
    const releaseFirst = editor.localEdits.stop();
    const releaseSecond = editor.localEdits.stop();
    releaseFirst();
    insert('blocked');
    editor.settlePendingEdits();
    expect(editor.proseText()).not.toContain('blocked');

    releaseSecond();
    insert('accepted');
    editor.settlePendingEdits();
    expect(editor.proseText()).toContain('accepted');
  });

  it('checks the owner input permission synchronously without releasing a document stop', async () => {
    await create();
    let allowed = true;
    editor.localEdits.isInputAllowed = () => allowed;
    insert('first');
    editor.settlePendingEdits();
    allowed = false;
    insert('blocked');
    editor.settlePendingEdits();
    expect(editor.proseText()).toBe('first');

    const release = editor.localEdits.stop();
    allowed = true;
    insert('still blocked');
    editor.settlePendingEdits();
    expect(editor.proseText()).toBe('first');
    release();
    insert(' accepted');
    editor.settlePendingEdits();
    expect(editor.proseText()).toBe('first accepted');
  });

  it('does not notify save observers for selection-only updates', async () => {
    await create();
    insert('text');
    editor.settlePendingEdits();
    const changed = vi.fn();
    editor.localEdits.onChange(changed);

    editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 2, end: 2 } });
    editor.settlePendingEdits();
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 3, end: 3 } }));
    expect(changed).not.toHaveBeenCalled();

    insert('changed');
    expect(changed).toHaveBeenCalled();
    editor.settlePendingEdits();
    expect(editor.localEdits.pending).toBe(false);
  });

  it('release does not override the document readonly state', async () => {
    await create();
    const release = editor.localEdits.stop();
    editor.readOnly = true;
    release();
    insert('blocked');
    editor.settlePendingEdits();
    expect(editor.proseText()).not.toContain('blocked');
    expect(editor.readOnly).toBe(true);
  });

  it.each(['textarea', 'edit-context'])('finalizes %s preedit exactly once before blocking input', async (mode) => {
    if (mode === 'textarea') vi.stubGlobal('EditContext', undefined);
    await create();
    target = document.createElement('div');
    document.body.append(target);
    mounted = mount(EditorFrameSyncTestHost, { target, props: { editor, userId: crypto.randomUUID() } });
    await tick();
    editor.updateNow((request) => request.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
    const input = editor.inputEl;
    if (!input) throw new Error('Input did not mount');
    input.focus();
    await tick();
    if (input instanceof HTMLTextAreaElement) {
      input.dispatchEvent(new CompositionEvent('compositionstart', { data: '', bubbles: true }));
      input.dispatchEvent(new CompositionEvent('compositionupdate', { data: '한글', bubbles: true }));
      input.dispatchEvent(
        new InputEvent('beforeinput', { inputType: 'insertCompositionText', data: '한글', bubbles: true, cancelable: true }),
      );
      // The browser accepted this value even if its input event has not run yet.
      input.setRangeText('한글', input.selectionStart, input.selectionEnd, 'end');
    } else {
      const context = input.editContext;
      if (!context) throw new Error('EditContext did not mount');
      context.dispatchEvent(new CompositionEvent('compositionstart'));
      const start = context.selectionStart;
      context.updateText(start, start, '한글');
      context.updateSelection(start + 2, start + 2);
      context.dispatchEvent(
        new TextUpdateEvent('textupdate', {
          updateRangeStart: start,
          updateRangeEnd: start,
          text: '한글',
          selectionStart: start + 2,
          selectionEnd: start + 2,
        }),
      );
    }
    expect(editor.localEdits.pending).toBe(true);
    editor.finalizeInput();
    const release = editor.localEdits.stop();
    expect(editor.localEdits.pending).toBe(false);
    expect(editor.proseText()).toContain('한글');
    if (input instanceof HTMLTextAreaElement) input.dispatchEvent(new CompositionEvent('compositionend', { data: '한글', bubbles: true }));
    else input.editContext?.dispatchEvent(new CompositionEvent('compositionend'));
    await tick();
    release();
    expect(editor.proseText().match(/한글/gu)).toHaveLength(1);
    expect(editor.localEdits.pending).toBe(false);
  });
});
