import '../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { Editor } from '../editor.svelte';
import EditorFrameSyncTestHost from '../editor-frame-sync-test-host.svelte';
import type { PlainDoc, PlainNode, PlainNodeEntry } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

const entry = (node: PlainNode, children: PlainNodeEntry[] = []): PlainNodeEntry => ({
  node,
  modifiers: {} as never,
  carry: [],
  children,
});

const doc = (text: string): PlainDoc => ({
  root: entry(
    {
      type: 'root',
      layout_mode: { type: 'continuous', max_width: 320 },
    },
    [entry({ type: 'paragraph' }, [entry({ type: 'text', text })])],
  ),
});

describe('web IME text replacement', () => {
  let editor: Editor | undefined;
  let mounted: Record<string, unknown> | undefined;

  const mountEditor = async (initialDoc = doc(''), position = 1): Promise<{ editor: Editor; input: HTMLTextAreaElement }> => {
    const host = await initWasm();
    host.set_text_replacement_rules([{ id: 'cry-to-laugh', matchPattern: 'ㅠㅠ', substitute: '하하하', regex: false }]);
    const mountedEditor = await Editor.createFromDoc(initialDoc, { width: 320, height: 180, scale_factor: 1 });
    editor = mountedEditor;

    const target = document.createElement('div');
    document.body.append(target);
    mounted = mount(EditorFrameSyncTestHost, {
      target,
      props: { editor: mountedEditor, userId: `ime-text-replacement-${crypto.randomUUID()}` },
    });
    await tick();
    mountedEditor.updateNow(() => {
      mountedEditor.enqueue({ type: 'selection', op: { type: 'set_flat', start: position, end: position } });
      mountedEditor.enqueue({ type: 'system', event: { type: 'set_focused', focused: true } });
    });
    await tick();

    const input = mountedEditor.inputEl;
    if (!input) throw new Error('Production editor input is not mounted');
    input.focus();
    return { editor: mountedEditor, input };
  };

  const imeEvents = (input: HTMLTextAreaElement) => ({
    insertText: (text: string) => {
      const accepted = input.dispatchEvent(
        new InputEvent('beforeinput', { inputType: 'insertText', data: text, bubbles: true, cancelable: true }),
      );
      expect(accepted).toBe(true);
      // Synthetic beforeinput does not perform the native textarea mutation.
      // The production input handler and WASM engine remain responsible for the edit.
      input.setRangeText(text, input.selectionStart, input.selectionEnd, 'end');
      input.dispatchEvent(new InputEvent('input', { inputType: 'insertText', data: text, bubbles: true }));
    },
    composition: (type: 'compositionstart' | 'compositionupdate' | 'compositionend', data: string) =>
      input.dispatchEvent(new CompositionEvent(type, { data, bubbles: true })),
    beforeCompositionInput: (data: string) =>
      input.dispatchEvent(
        new InputEvent('beforeinput', {
          inputType: 'insertCompositionText',
          data,
          isComposing: true,
          bubbles: true,
          cancelable: true,
        }),
      ),
    beforeTextInput: (data: string) =>
      input.dispatchEvent(
        new InputEvent('beforeinput', {
          inputType: 'insertText',
          data,
          isComposing: true,
          bubbles: true,
          cancelable: true,
        }),
      ),
    applyNativeInput: (value: string, selection: number, inputType: string, data: string) => {
      input.value = value;
      input.setSelectionRange(selection, selection);
      input.dispatchEvent(new InputEvent('input', { inputType, data, isComposing: true, bubbles: true }));
    },
    composingKeyDown: (key: string) =>
      input.dispatchEvent(new KeyboardEvent('keydown', { key, isComposing: true, bubbles: true, cancelable: true })),
  });

  afterEach(async () => {
    if (mounted) await unmount(mounted);
    mounted = undefined;
    editor?.destroy();
    editor = undefined;
    const host = await initWasm();
    host.set_text_replacement_rules([]);
    document.body.replaceChildren();
  });

  it.each([
    // Prose export separates paragraphs with a blank line; IME text preserves every block boundary.
    { text: 'a\nb', prose: 'a\n\nb', buffer: '\u{2028}a\u{2029}\u{2028}b\u{2029}', caret: 5 },
    { text: 'a\r\n\r😀b', prose: 'a\n\n😀b', buffer: '\u{2028}a\u{2029}\u{2028}\u{2029}\u{2028}😀b\u{2029}', caret: 8 },
  ])('syncs paragraph coordinates after native insertion of $text', async ({ text, prose, buffer, caret }) => {
    const { editor, input } = await mountEditor();
    const events = imeEvents(input);

    events.insertText(text);
    await tick();

    expect(editor.proseText()).toBe(prose);
    expect(editor.ime(100, 100)).toMatchObject({ text: buffer, selection: { start: caret, end: caret } });
    expect(input.value).toBe(buffer);
    expect(input.selectionStart).toBe(buffer.length - 1);
    expect(input.selectionEnd).toBe(buffer.length - 1);

    events.insertText('X');
    await tick();

    expect(editor.proseText()).toBe(`${prose}X`);
    expect(editor.ime(100, 100)?.selection).toEqual({ start: caret + 1, end: caret + 1 });
    expect(input.value).toBe(`${buffer.slice(0, -1)}X\u{2029}`);
  });

  it('composes and converts Japanese in the paragraph created by a native multiline insertion', async () => {
    const { editor, input } = await mountEditor();
    const events = imeEvents(input);

    events.insertText('a\nb');
    await tick();
    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'に');
    events.beforeCompositionInput('に');
    events.applyNativeInput('\u{2028}a\u{2029}\u{2028}bに\u{2029}', 6, 'insertCompositionText', 'に');
    await tick();

    expect(editor.proseText()).toBe('a\n\nbに');
    expect(editor.ime(100, 100)?.composing).toEqual({ start: 5, end: 6 });

    events.composition('compositionupdate', '日本語');
    events.beforeCompositionInput('日本語');
    events.applyNativeInput('\u{2028}a\u{2029}\u{2028}b日本語\u{2029}', 8, 'insertCompositionText', '日本語');
    events.composition('compositionend', '日本語');
    await tick();
    events.insertText('X');
    await tick();

    expect(editor.proseText()).toBe('a\n\nb日本語X');
    expect(editor.ime(100, 100)).toMatchObject({ selection: { start: 9, end: 9 }, composing: undefined });
    expect(input.value).toBe('\u{2028}a\u{2029}\u{2028}b日本語X\u{2029}');
    expect(input.selectionStart).toBe(9);
  });

  it('keeps Japanese clauses on the same native input line at the editor caret height', async () => {
    const { input } = await mountEditor();
    const events = imeEvents(input);
    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'に');
    events.beforeCompositionInput('に');
    events.applyNativeInput('\u{2028}に\u{2029}', 2, 'insertCompositionText', 'に');
    await tick();
    const initialScrollHeight = input.scrollHeight;

    events.composition('compositionupdate', '日本の国');
    events.beforeCompositionInput('日本の国');
    events.applyNativeInput('\u{2028}日本の国\u{2029}', 5, 'insertCompositionText', '日本の国');
    await tick();

    // Extra clauses must extend horizontally, without moving their candidate anchors
    // onto different textarea lines. The surrounding paragraph separators remain.
    expect(input.scrollHeight).toBe(initialScrollHeight);
    const rect = input.getBoundingClientRect();
    const style = getComputedStyle(input);
    expect(Number.parseFloat(style.lineHeight)).toBeCloseTo(rect.height, 1);
    expect(Number.parseFloat(style.fontSize)).toBeCloseTo(rect.height, 1);
  });

  it('aligns the native input scroll with programmatic caret movement in a long line', async () => {
    const { editor, input } = await mountEditor(doc('Long text before the caret '.repeat(5)), 100);
    const nativePrefixWidth = () => {
      const probe = input.cloneNode() as HTMLTextAreaElement;
      probe.value = input.value.slice(0, input.selectionEnd);
      input.after(probe);
      const width = probe.scrollWidth;
      probe.remove();
      return width;
    };
    await expect.poll(() => input.getBoundingClientRect().left).toBeGreaterThan(-1000);
    await expect.poll(() => input.scrollLeft).toBeGreaterThan(100);
    expect(Math.abs(input.scrollLeft - nativePrefixWidth())).toBeLessThanOrEqual(1);

    editor.updateNow(() => {
      editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
    });
    await tick();

    expect(input.selectionStart).toBe(1);
    expect(input.scrollLeft).toBeLessThan(10);
    expect(Math.abs(input.scrollLeft - nativePrefixWidth())).toBeLessThanOrEqual(1);
  });

  it('replaces a selection with multiple paragraphs and keeps the suffix after the caret', async () => {
    const { editor, input } = await mountEditor(doc('leftOLDright'), 5);
    const events = imeEvents(input);

    input.setSelectionRange(5, 8);
    events.insertText('a\nb');
    await tick();
    expect(editor.proseText()).toBe('lefta\n\nbright');
    expect(input.selectionStart).toBe(9);
    expect(input.selectionEnd).toBe(9);

    events.insertText('X');
    await tick();
    expect(editor.proseText()).toBe('lefta\n\nbXright');
    expect(editor.ime(100, 100)?.selection).toEqual({ start: 10, end: 10 });
  });

  it('materializes paragraphs before an image and preserves the following text during native input', async () => {
    const initialDoc = doc('tail');
    initialDoc.root.children.unshift(entry({ type: 'image', id: 'asset', proportion: 100 }));
    const { editor, input } = await mountEditor(initialDoc, 0);
    const events = imeEvents(input);
    editor.updateNow(() => {
      editor.enqueue({ type: 'navigation', op: { type: 'move', movement: { type: 'document', direction: 'backward' }, extend: false } });
    });
    await tick();
    expect(editor.appliedSnapshot.selection?.head).toMatchObject({ offset: 0, affinity: 'upstream' });

    events.insertText('a\nb');
    await tick();
    events.insertText('X');
    await tick();

    expect(editor.proseText()).toBe('a\n\nbX\n\ntail');
    expect(editor.appliedSnapshot.externalElements.map(({ data }) => data)).toEqual([{ type: 'image', id: 'asset', proportion: 100 }]);
    expect(editor.ime(100, 100)?.selection).toEqual({ start: 6, end: 6 });
    expect(input.selectionStart).toBe(6);
  });

  it('resolves input-buffer selection and composition across a commit barrier in the real WASM engine', async () => {
    const { editor, input } = await mountEditor();

    // Browser input events do not expose Compose's multi-command batch. Exercise
    // that shared contract separately, including the shipped WASM FFI boundary.
    editor.updateNow(() => {
      editor.enqueue({
        type: 'text_input',
        ops: [
          { type: 'replace_selection', text: 'a\n😀b' },
          { type: 'commit_as_is' },
          { type: 'set_composition', start: 4, end: 5 },
          { type: 'compose', text: 'に' },
          { type: 'set_selection', start: 4, end: 4 },
        ],
      });
    });
    await tick();

    expect(editor.ime(100, 100)).toMatchObject({
      text: '\u{2028}a\u{2029}\u{2028}😀に\u{2029}',
      selection: { start: 5, end: 5 },
      composing: { start: 5, end: 6 },
    });
    expect(input.value).toBe('\u{2028}a\u{2029}\u{2028}😀に\u{2029}');
    expect(input.selectionStart).toBe(6);
    expect(input.selectionEnd).toBe(6);
  });

  it('replaces a Korean match when macOS appends Space to the final composition update', async () => {
    const { editor, input } = await mountEditor();
    const events = imeEvents(input);

    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'ㅠ');
    events.beforeCompositionInput('ㅠ');
    events.applyNativeInput('\u{2028}ㅠ\u{2029}', 2, 'insertCompositionText', 'ㅠ');

    events.composingKeyDown('ㅠ');
    events.composition('compositionupdate', 'ㅠ');
    events.beforeCompositionInput('ㅠ');
    events.applyNativeInput('\u{2028}ㅠ\u{2029}', 2, 'insertCompositionText', 'ㅠ');
    events.composition('compositionend', 'ㅠ');
    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'ㅠ');
    events.beforeCompositionInput('ㅠ');
    events.applyNativeInput('\u{2028}ㅠㅠ\u{2029}', 3, 'insertCompositionText', 'ㅠ');

    events.composingKeyDown(' ');
    events.composition('compositionupdate', 'ㅠ ');
    events.beforeCompositionInput('ㅠ ');
    events.applyNativeInput('\u{2028}ㅠㅠ \u{2029}', 4, 'insertCompositionText', 'ㅠ ');
    events.composition('compositionend', 'ㅠ ');

    await new Promise(requestAnimationFrame);
    expect(editor.proseText()).toBe('하하하 ');
  });

  it('inserts text after a replacement when insertText arrives before compositionend', async () => {
    const { editor, input } = await mountEditor();
    const events = imeEvents(input);

    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'ㅠ');
    events.beforeCompositionInput('ㅠ');
    events.applyNativeInput('\u{2028}ㅠ\u{2029}', 2, 'insertCompositionText', 'ㅠ');
    events.composition('compositionend', 'ㅠ');

    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'ㅠ');
    events.beforeCompositionInput('ㅠ');
    events.applyNativeInput('\u{2028}ㅠㅠ\u{2029}', 3, 'insertCompositionText', 'ㅠ');

    if (events.beforeTextInput(' ')) {
      events.applyNativeInput('\u{2028}ㅠㅠ \u{2029}', 4, 'insertText', ' ');
    }
    events.composition('compositionend', 'ㅠ');

    await new Promise(requestAnimationFrame);
    expect(editor.proseText()).toBe('하하하 ');
  });

  it('applies a deferred composition tail and its following edit in one input admission', async () => {
    const { editor, input } = await mountEditor();
    const events = imeEvents(input);

    events.composition('compositionstart', '');
    events.composition('compositionupdate', 'にほ');
    events.beforeCompositionInput('にほ');
    events.applyNativeInput('\u{2028}にほ\u{2029}', 3, 'insertCompositionText', 'にほ');

    events.composingKeyDown('n');
    events.composition('compositionupdate', 'にほn');
    events.beforeCompositionInput('にほn');
    events.applyNativeInput('\u{2028}にほn\u{2029}', 4, 'insertCompositionText', 'にほn');

    events.applyNativeInput('\u{2028}にほん\u{2029}', 4, 'insertCompositionText', 'にほん');

    await new Promise(requestAnimationFrame);
    expect(editor.proseText()).toBe('にほん');
  });
});
