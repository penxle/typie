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

  const mountEditor = async (): Promise<{ editor: Editor; input: HTMLTextAreaElement }> => {
    const host = await initWasm();
    host.set_text_replacement_rules([{ id: 'cry-to-laugh', matchPattern: 'ㅠㅠ', substitute: '하하하', regex: false }]);
    const mountedEditor = await Editor.createFromDoc(doc(''), { width: 320, height: 180, scale_factor: 1 });
    editor = mountedEditor;

    const target = document.createElement('div');
    document.body.append(target);
    mounted = mount(EditorFrameSyncTestHost, {
      target,
      props: { editor: mountedEditor, userId: `ime-text-replacement-${crypto.randomUUID()}` },
    });
    await tick();
    mountedEditor.updateNow(() => {
      mountedEditor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } });
      mountedEditor.enqueue({ type: 'system', event: { type: 'set_focused', focused: true } });
    });
    await tick();

    const input = mountedEditor.inputEl;
    if (!input) throw new Error('Production editor input is not mounted');
    input.focus();
    return { editor: mountedEditor, input };
  };

  const imeEvents = (input: HTMLTextAreaElement) => ({
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
