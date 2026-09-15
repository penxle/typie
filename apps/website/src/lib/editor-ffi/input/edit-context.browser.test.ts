import '../../../app.css';

import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { cdp, userEvent } from 'vitest/browser';
import { initWasm } from '$lib/wasm-ffi.svelte';
import { Editor } from '../editor.svelte';
import EditorFrameSyncTestHost from '../editor-frame-sync-test-host.svelte';
import type { PlainDoc } from '@typie/editor-ffi/browser';
import type { CDPSession } from '@vitest/browser-playwright';

vi.mock('$env/dynamic/public', () => ({ env: {} }));

describe('EditContext input', () => {
  let editor: Editor;
  let mounted: Record<string, unknown>;

  const mountEditor = async (text = '') => {
    await initWasm();
    const doc: PlainDoc = {
      root: {
        node: { type: 'root', layout_mode: { type: 'continuous', max_width: 320 } },
        modifiers: {} as never,
        carry: [],
        children: [
          {
            node: { type: 'paragraph' },
            modifiers: {} as never,
            carry: [],
            children: [{ node: { type: 'text', text }, modifiers: {} as never, carry: [], children: [] }],
          },
        ],
      },
    };
    editor = await Editor.createFromDoc(doc, { width: 320, height: 180, scale_factor: 1 });
    const target = document.createElement('div');
    document.body.append(target);
    mounted = mount(EditorFrameSyncTestHost, { target, props: { editor, userId: crypto.randomUUID() } });
    await tick();
    editor.updateNow(() => editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 1, end: 1 } }));
    editor.inputEl?.focus();
    await tick();
    const context = editor.inputEl?.editContext;
    expect(context, 'supported browsers use the production EditContext input').toBeInstanceOf(EditContext);
    if (!context) throw new Error('EditContext input is not mounted');
    return context;
  };

  // Synthetic TextUpdateEvent does not mutate the browser's EditContext buffer.
  const input = (
    context: EditContext,
    start: number,
    end: number,
    text: string,
    selectionStart = start + text.length,
    selectionEnd = selectionStart,
  ) => {
    context.updateText(start, end, text);
    context.updateSelection(selectionStart, selectionEnd);
    context.dispatchEvent(
      new TextUpdateEvent('textupdate', {
        updateRangeStart: start,
        updateRangeEnd: end,
        text,
        selectionStart,
        selectionEnd,
      }),
    );
  };

  const waitForFrame = async () => {
    await tick();
    await expect.poll(() => editor.isPublished(editor.appliedRevision, { requireFrame: true })).toBe(true);
  };

  const targetClause = (context: EditContext, start: number, end: number) => {
    context.dispatchEvent(
      new TextFormatUpdateEvent('textformatupdate', {
        textFormats: [new TextFormat({ rangeStart: start, rangeEnd: end, underlineStyle: 'solid', underlineThickness: 'thick' })],
      }),
    );
  };

  const backgroundAt = (offset: number) => {
    const rect = editor.firstRectForRange(offset, offset + 1);
    if (!rect) throw new Error('Editor character is not rendered');
    const surface = document.querySelector(`[data-page-surface="${rect.page_idx}"]`);
    // Sample the character's left padding inside its rendered tile, excluding line spacing.
    const x = Math.floor((rect.rect.x + 1) * editor.surfaceScaleFactor);
    const y = Math.floor((rect.rect.y + rect.rect.height / 2) * editor.surfaceScaleFactor);
    const canvas = [...(surface?.querySelectorAll('canvas') ?? [])].find((tile) => {
      const left = Number(tile.dataset.tileX);
      const top = Number(tile.dataset.tileY);
      return x >= left && y >= top && x < left + tile.width && y < top + tile.height;
    });
    const context = canvas?.getContext('2d');
    if (!canvas || !context) throw new Error('Editor page is not rendered');
    return [...context.getImageData(x - Number(canvas.dataset.tileX), y - Number(canvas.dataset.tileY), 1, 1).data];
  };

  afterEach(async () => {
    if (mounted) await unmount(mounted);
    editor?.destroy();
    document.body.replaceChildren();
  });

  it('replaces UTF-16 ranges and resyncs structural offsets after a multiline insertion', async () => {
    const context = await mountEditor('😀old');
    input(context, 3, 6, 'a\nb');
    await tick();
    expect(editor.proseText()).toBe('😀a\n\nb');
    expect(context.text).toBe('\u{2028}😀a\u{2029}\u{2028}b\u{2029}');
    input(context, context.selectionStart, context.selectionEnd, 'X');
    await tick();
    expect(editor.proseText()).toBe('😀a\n\nbX');
  });

  it.each([' ', '.', '。', '/'])('replaces Korean text before the final composition separator %j', async (separator) => {
    const host = await initWasm();
    host.set_text_replacement_rules([{ id: 'laugh', matchPattern: 'ㅎㅎ', substitute: '웃음소리', regex: false }]);
    const context = await mountEditor('ㅎ');
    editor.updateNow(() => editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 2, end: 2 } }));
    await tick();
    context.dispatchEvent(new CompositionEvent('compositionstart'));
    input(context, 2, 2, 'ㅎ');
    // EditContext can report isComposing=false even while native preedit is active.
    editor.inputEl?.dispatchEvent(new KeyboardEvent('keydown', { key: separator, isComposing: false, bubbles: true, cancelable: true }));
    input(context, 2, 3, `ㅎ${separator}`);
    context.dispatchEvent(new CompositionEvent('compositionend', { data: `ㅎ${separator}` }));
    await tick();
    expect(editor.proseText()).toBe(`웃음소리${separator}`);
    expect(context.text).toBe(`\u{2028}웃음소리${separator}\u{2029}`);
    await userEvent.keyboard('{Backspace}');
    expect(editor.proseText()).toBe(separator === ' ' ? 'ㅎㅎ' : `ㅎㅎ${separator}`);
  });

  it.each([' ', '.', '👩‍💻'])('replaces native Korean composition ending in %j without a printable keydown', async (separator) => {
    const host = await initWasm();
    host.set_text_replacement_rules([{ id: 'laugh', matchPattern: 'ㅎㅎ', substitute: '웃음소리', regex: false }]);
    const context = await mountEditor('ㅎ');
    editor.updateNow(() => editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 2, end: 2 } }));
    await tick();
    const browser = cdp() as CDPSession;
    await browser.send('Input.imeSetComposition', { text: 'ㅎ', selectionStart: 1, selectionEnd: 1 });
    await browser.send('Input.insertText', { text: `ㅎ${separator}` });
    await expect.poll(() => editor.ime(64, 64)?.composing).toBeUndefined();
    expect(editor.proseText()).toBe(`웃음소리${separator}`);
    expect(context.text).toBe(`\u{2028}웃음소리${separator}\u{2029}`);
    await userEvent.keyboard('{Backspace}');
    expect(editor.proseText()).toBe(separator === ' ' ? 'ㅎㅎ' : `ㅎㅎ${separator}`);
  });

  it.each([false, true])('keeps an appended key in preedit when composition continues (next update: %s)', async (nextUpdate) => {
    const host = await initWasm();
    host.set_text_replacement_rules([
      { id: 'prefix', matchPattern: 'にほ', substitute: 'wrong', regex: false },
      { id: 'complete', matchPattern: nextUpdate ? 'にほん' : 'にほn', substitute: 'complete', regex: false },
    ]);
    const context = await mountEditor();
    context.dispatchEvent(new CompositionEvent('compositionstart'));
    input(context, 1, 1, 'にほ');
    editor.inputEl?.dispatchEvent(new KeyboardEvent('keydown', { key: 'n', bubbles: true, cancelable: true }));
    input(context, 1, 3, 'にほn');
    if (nextUpdate) input(context, 1, 4, 'にほん');
    await expect.poll(() => editor.proseText()).toBe(nextUpdate ? 'にほん' : 'にほn');
    context.dispatchEvent(new CompositionEvent('compositionend'));
    await tick();
    expect(editor.proseText()).toBe('complete');
  });

  it('keeps the active Japanese clause and updates its decoration without changing text', async () => {
    const context = await mountEditor();
    context.dispatchEvent(new CompositionEvent('compositionstart'));
    input(context, 1, 1, '日本の国', 1, 4);
    await waitForFrame();
    const plain = backgroundAt(1);
    expect(backgroundAt(4)).toEqual(plain);
    context.dispatchEvent(
      new TextFormatUpdateEvent('textformatupdate', {
        textFormats: [
          new TextFormat({ rangeStart: 1, rangeEnd: 4, underlineStyle: 'solid', underlineThickness: 'thick' }),
          new TextFormat({ rangeStart: 4, rangeEnd: 5, underlineStyle: 'solid', underlineThickness: 'thin' }),
        ],
      }),
    );
    await waitForFrame();
    expect(editor.proseText()).toBe('日本の国');
    expect(context.selectionStart).toBe(1);
    expect(context.selectionEnd).toBe(4);
    const selected = backgroundAt(1);
    expect(selected).not.toEqual(plain);
    expect(backgroundAt(4)).toEqual(plain);

    input(context, 1, 5, '日本の国', 4, 5);
    context.dispatchEvent(
      new TextFormatUpdateEvent('textformatupdate', {
        textFormats: [
          new TextFormat({ rangeStart: 1, rangeEnd: 4, underlineStyle: 'solid', underlineThickness: 'thin' }),
          new TextFormat({ rangeStart: 4, rangeEnd: 5, underlineStyle: 'solid', underlineThickness: 'thick' }),
        ],
      }),
    );
    await waitForFrame();
    expect(context.selectionStart).toBe(4);
    expect(context.selectionEnd).toBe(5);
    expect(editor.proseText()).toBe('日本の国');
    expect(backgroundAt(1)).toEqual(plain);
    expect(backgroundAt(4)).toEqual(selected);

    context.dispatchEvent(
      new TextFormatUpdateEvent('textformatupdate', {
        textFormats: [new TextFormat({ rangeStart: 1, rangeEnd: 5, underlineStyle: 'solid', underlineThickness: 'thick' })],
      }),
    );
    await waitForFrame();
    expect(backgroundAt(1)).toEqual(plain);
    expect(backgroundAt(4)).toEqual(plain);

    context.dispatchEvent(new CompositionEvent('compositionend', { data: '日本の国' }));
    await waitForFrame();
    expect(editor.ime(64, 64)?.composing).toBeUndefined();
    expect(backgroundAt(4)).toEqual(plain);
  });

  it.each(['thin', 'thick'] as const)('leaves a whole %s composition without a background', async (underlineThickness) => {
    const context = await mountEditor('before after');
    context.dispatchEvent(new CompositionEvent('compositionstart'));
    input(context, 8, 8, '日本', 8, 10);
    await waitForFrame();
    const plain = backgroundAt(8);
    context.dispatchEvent(
      new TextFormatUpdateEvent('textformatupdate', {
        textFormats: [new TextFormat({ rangeStart: 8, rangeEnd: 10, underlineStyle: 'solid', underlineThickness })],
      }),
    );
    await waitForFrame();
    expect(editor.proseText()).toBe('before 日本after');
    expect(editor.ime(64, 64)?.composing).toEqual({ start: 8, end: 10 });
    expect(backgroundAt(8)).toEqual(plain);
  });

  it('handles browser-generated composition, bounds requests, commit, and subsequent keyboard input', async () => {
    const context = await mountEditor();
    const browser = cdp() as CDPSession;
    await browser.send('Input.imeSetComposition', { text: 'にほんのくに', selectionStart: 6, selectionEnd: 6 });
    await expect.poll(() => editor.proseText()).toBe('にほんのくに');
    await browser.send('Input.imeSetComposition', { text: '日本の国', selectionStart: 3, selectionEnd: 4 });
    await expect.poll(() => editor.proseText()).toBe('日本の国');
    await expect.poll(() => context.characterBounds().length).toBe(4);
    expect(context.characterBounds().every((rect) => rect.width > 0 && rect.height > 0)).toBe(true);
    expect(context.selectionStart).toBe(4);
    expect(context.selectionEnd).toBe(5);
    await browser.send('Input.insertText', { text: '日本の国' });
    await expect.poll(() => editor.ime(64, 64)?.composing).toBeUndefined();
    await userEvent.keyboard('X{ArrowLeft}{Backspace}');
    await expect.poll(() => editor.proseText()).toBe('日本のX');
  });

  it('preserves the insertion position when a selection-only update follows composition commit', async () => {
    const prefix = '앞쪽 본문입니다. '.repeat(20);
    const suffix = '뒤쪽 본문';
    const context = await mountEditor(prefix + suffix);
    editor.updateNow(() =>
      editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: prefix.length + 1, end: prefix.length + 1 } }),
    );
    await tick();
    const browser = cdp() as CDPSession;
    await browser.send('Input.imeSetComposition', { text: '日本', selectionStart: 2, selectionEnd: 2 });
    await browser.send('Input.insertText', { text: '日本' });
    await tick();
    expect(editor.proseText()).toBe(prefix + '日本' + suffix);
    const committedSelection = context.selectionStart;
    context.dispatchEvent(
      new TextUpdateEvent('textupdate', {
        updateRangeStart: 0,
        updateRangeEnd: 0,
        text: '',
        selectionStart: committedSelection,
        selectionEnd: committedSelection,
      }),
    );
    await tick();
    await browser.send('Input.imeSetComposition', { text: '語', selectionStart: 1, selectionEnd: 1 });
    expect(editor.proseText()).toBe(prefix + '日本語' + suffix);

    await browser.send('Input.insertText', { text: '語' });
    await tick();
    const end = context.selectionEnd;
    input(context, 0, 0, '', end - 1, end);
    await tick();
    await browser.send('Input.imeSetComposition', { text: '文', selectionStart: 1, selectionEnd: 1 });
    expect(editor.proseText()).toBe(prefix + '日本文' + suffix);
  });

  it('preserves the active composition when only the native selection is updated', async () => {
    const context = await mountEditor('before after');
    editor.updateNow(() => editor.enqueue({ type: 'selection', op: { type: 'set_flat', start: 8, end: 8 } }));
    await tick();
    const browser = cdp() as CDPSession;
    await browser.send('Input.imeSetComposition', { text: '日本', selectionStart: 0, selectionEnd: 1 });
    const composition = editor.ime(64, 64)?.composing;
    context.dispatchEvent(
      new TextUpdateEvent('textupdate', {
        updateRangeStart: 0,
        updateRangeEnd: 0,
        text: '',
        selectionStart: context.selectionStart,
        selectionEnd: context.selectionEnd,
      }),
    );
    await tick();
    expect(editor.ime(64, 64)?.composing).toEqual(composition);
    await browser.send('Input.imeSetComposition', { text: '日本語', selectionStart: 3, selectionEnd: 3 });
    expect(editor.proseText()).toBe('before 日本語after');
  });

  it('cancels native preedit without leaving text or decorations, and commits on blur', async () => {
    const context = await mountEditor('suffix');
    const browser = cdp() as CDPSession;
    await waitForFrame();
    const plain = backgroundAt(1);
    await browser.send('Input.imeSetComposition', { text: 'にほん', selectionStart: 3, selectionEnd: 3 });
    targetClause(context, 1, 2);
    await waitForFrame();
    expect(backgroundAt(1)).not.toEqual(plain);
    await browser.send('Input.imeSetComposition', { text: '', selectionStart: 0, selectionEnd: 0 });
    await expect.poll(() => editor.proseText()).toBe('suffix');
    await waitForFrame();
    expect(backgroundAt(1)).toEqual(plain);

    await browser.send('Input.imeSetComposition', { text: '日本', selectionStart: 0, selectionEnd: 2 });
    targetClause(context, 1, 2);
    await waitForFrame();
    expect(backgroundAt(1)).not.toEqual(plain);
    editor.inputEl?.blur();
    await expect.poll(() => editor.ime(64, 64)?.composing).toBeUndefined();
    expect(editor.proseText()).toBe('日本suffix');
    await waitForFrame();
    expect(backgroundAt(1)).toEqual(plain);
    editor.focus();
    await tick();
    expect(context.text).toBe('\u{2028}日本suffix\u{2029}');
  });

  it('supplies the same character bounds for both UTF-16 halves of an emoji', async () => {
    const context = await mountEditor();
    await (cdp() as CDPSession).send('Input.imeSetComposition', { text: '日😀本', selectionStart: 1, selectionEnd: 3 });
    await expect.poll(() => context.characterBounds().length).toBe(4);
    const bounds = context.characterBounds();
    expect(bounds[1].toJSON()).toEqual(bounds[2].toJSON());
    expect(bounds[1].x).toBeGreaterThan(bounds[0].x);
    expect(bounds[3].x).toBeGreaterThan(bounds[2].x);
  });

  it('ends composition without editing text when input becomes read-only', async () => {
    const context = await mountEditor();
    await (cdp() as CDPSession).send('Input.imeSetComposition', { text: '日本', selectionStart: 0, selectionEnd: 1 });
    await waitForFrame();
    const plain = backgroundAt(1);
    targetClause(context, 1, 2);
    await waitForFrame();
    expect(backgroundAt(1)).not.toEqual(plain);
    editor.readOnly = true;
    await waitForFrame();
    expect(editor.inputEl?.editContext).toBeNull();
    expect(editor.ime(64, 64)?.composing).toBeUndefined();
    expect(editor.proseText()).toBe('日本');
    expect(backgroundAt(1)).toEqual(plain);
    editor.updateNow(() =>
      editor.enqueue({ type: 'text_input', ops: [{ type: 'clear_composition' }, { type: 'replace_selection', text: 'blocked' }] }),
    );
    expect(editor.proseText()).toBe('日本');
    editor.readOnly = false;
    await tick();
    await (cdp() as CDPSession).send('Input.imeSetComposition', { text: '語', selectionStart: 1, selectionEnd: 1 });
    expect(editor.proseText()).toBe('日本語');
  });

  it('recovers from an engine IME rejection without replaying the detached native composition', async () => {
    const context = await mountEditor();
    const browser = cdp() as CDPSession;
    await browser.send('Input.imeSetComposition', { text: '日本', selectionStart: 2, selectionEnd: 2 });
    await waitForFrame();
    const plain = backgroundAt(1);
    targetClause(context, 1, 2);
    await waitForFrame();
    expect(backgroundAt(1)).not.toEqual(plain);
    editor.updateNow(() =>
      editor.enqueue({
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 0, end: 0 },
          { type: 'replace_selection', text: 'invalid' },
        ],
      }),
    );
    await tick();
    expect(editor.proseText()).toBe('日本');
    expect(editor.ime(64, 64)?.composing).toBeUndefined();
    expect(context.text).toBe('\u{2028}日本\u{2029}');
    await waitForFrame();
    expect(backgroundAt(1)).toEqual(plain);
    await browser.send('Input.insertText', { text: 'X' });
    await expect.poll(() => editor.proseText()).toBe('日本X');
  });

  it('commits native composition before a page-break shortcut', async () => {
    await mountEditor();
    const browser = cdp() as CDPSession;
    await browser.send('Input.imeSetComposition', { text: '日本', selectionStart: 2, selectionEnd: 2 });
    await userEvent.keyboard('{Meta>}{Enter}{/Meta}');
    await browser.send('Input.insertText', { text: '日本' });
    await expect.poll(() => editor.ime(64, 64)?.composing).toBeUndefined();
  });

  it.each([false, true])('pastes after committing Korean preedit once (text only: %s)', async (textOnly) => {
    const context = await mountEditor('뒤');
    const browser = cdp() as CDPSession;
    const read = vi
      .spyOn(navigator.clipboard, 'read')
      .mockResolvedValue([new ClipboardItem({ 'text/plain': new Blob(['ABC'], { type: 'text/plain' }) })]);
    try {
      await browser.send('Input.imeSetComposition', { text: '한', selectionStart: 1, selectionEnd: 1 });
      await userEvent.keyboard(textOnly ? '{Meta>}{Shift>}v{/Shift}{/Meta}' : '{Meta>}v{/Meta}');
      // macOS delivers the native commit after the consumed paste keydown.
      await browser.send('Input.insertText', { text: '한' });
      await expect.poll(() => editor.proseText()).toBe('한ABC뒤');
      expect(editor.ime(64, 64)?.composing).toBeUndefined();
      expect(context.text).toBe('\u{2028}한ABC뒤\u{2029}');
      expect(document.activeElement).toBe(editor.inputEl);
      await browser.send('Input.imeSetComposition', { text: '한', selectionStart: 1, selectionEnd: 1 });
      await browser.send('Input.insertText', { text: '한' });
      expect(editor.proseText()).toBe('한ABC한뒤');
    } finally {
      read.mockRestore();
    }
  });
});
