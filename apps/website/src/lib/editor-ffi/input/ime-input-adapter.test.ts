// cspell:ignore aeeeeeb aeeeeeeb xeeeeeb eeeeeeb xeeeeeeb

import { describe, expect, it, vi } from 'vitest';
import { ImeInputAdapter } from './ime-input-adapter';
import { beforeInputEvent, compositionEvent, context, inputEvent } from './ime-test-fixtures';
import type { Message } from '@typie/editor-ffi/browser';
import type { ImeContext } from './ime-context';

const createInput = () => document.createElement('textarea');

const createImeHarness = (initialContext: ImeContext) => {
  const input = createInput();
  const messages: Message[] = [];
  let editorContext = initialContext;
  const adapter = new ImeInputAdapter({
    readContext: () => editorContext,
    enqueue: (next) => {
      messages.push(...next);
    },
  });

  return {
    input,
    messages,
    adapter,
    setContext: (context: ImeContext) => {
      editorContext = context;
    },
    syncFromEditor: () => adapter.syncFromEditor(input),
    compositionStart: (data = '') => adapter.handleCompositionStart(compositionEvent(input, data)),
    compositionUpdate: (data: string) => adapter.handleCompositionUpdate(compositionEvent(input, data)),
    compositionEnd: () => adapter.handleCompositionEnd(),
    beforeCompositionInput: (text: string) => adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', text)),
    beforeTextInput: (text: string) => {
      const event = beforeInputEvent(input, 'insertText', text);
      adapter.handleBeforeInput(event);
      return event;
    },
    applyNativeInput: (value: string, selectionStart: number, selectionEnd = selectionStart) => {
      input.value = value;
      input.setSelectionRange(selectionStart, selectionEnd);
      adapter.handleInput(inputEvent(input));
    },
    expectInput: (value: string, selectionStart: number, selectionEnd = selectionStart) => {
      expect(input.value).toBe(value);
      expect(input.selectionStart).toBe(selectionStart);
      expect(input.selectionEnd).toBe(selectionEnd);
    },
  };
};

describe('ImeInputAdapter', () => {
  it('captures context during beforeinput and diffs the following native input mutation', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => context(''),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    const beforeInput = beforeInputEvent(input, 'insertText', 'a');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).not.toHaveBeenCalled();
    expect(input.value).toBe('');

    input.value = 'a';
    input.setSelectionRange(1, 1);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 20, end: 20 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
    ]);
  });

  it('splits a multi-line text insertion into paragraph splits via enter keys', () => {
    const harness = createImeHarness(context(''));

    harness.beforeTextInput('a\nb');
    harness.applyNativeInput('a\nb', 3);

    expect(harness.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 20, end: 20 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
      { type: 'key', event: { key: 'enter' } },
      {
        type: 'text_input',
        ops: [{ type: 'replace_selection', text: 'b' }],
      },
    ]);
  });

  it('normalizes carriage returns and keeps empty lines in multi-line insertions', () => {
    const harness = createImeHarness(context(''));

    harness.beforeTextInput('a\r\n\rb');
    harness.applyNativeInput('a\r\n\rb', 5);

    expect(harness.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 20, end: 20 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
      { type: 'key', event: { key: 'enter' } },
      { type: 'key', event: { key: 'enter' } },
      {
        type: 'text_input',
        ops: [{ type: 'replace_selection', text: 'b' }],
      },
    ]);
  });

  it('anchors repeated-character insertions to the beforeinput collapsed selection', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: 'aeeeeeb',
        windowStart: 50,
        selection: { start: 53, end: 53 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    expect(input.selectionStart).toBe(3);

    const beforeInput = beforeInputEvent(input, 'insertText', 'e');
    adapter.handleBeforeInput(beforeInput);

    input.value = 'aeeeeeeb';
    input.setSelectionRange(4, 4);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 53, end: 53 },
          { type: 'replace_selection', text: 'e' },
        ],
      },
    ]);
  });

  it('does not rewrite the native input when editor sync only rebases the same local IME window', () => {
    const input = createInput();
    const messages: Message[] = [];
    let syncCount = 0;
    const adapter = new ImeInputAdapter({
      readContext: () => {
        syncCount += 1;
        return syncCount === 1
          ? {
              text: 'xeeeeeb',
              windowStart: 90,
              selection: { start: 94, end: 94 },
              composing: null,
            }
          : {
              text: 'eeeeeeb',
              windowStart: 91,
              selection: { start: 95, end: 95 },
              composing: null,
            };
      },
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    expect(input.value).toBe('xeeeeeb');
    expect(input.selectionStart).toBe(4);

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', 'e'));
    input.value = 'xeeeeeeb';
    input.setSelectionRange(5, 5);
    adapter.handleInput(inputEvent(input));

    adapter.syncFromEditor(input);

    expect(input.value).toBe('xeeeeeeb');
    expect(input.selectionStart).toBe(5);
    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 94, end: 94 },
          { type: 'replace_selection', text: 'e' },
        ],
      },
    ]);
  });

  it('uses the native DOM diff when the following native input appended', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}e\u{2029}',
        windowStart: 0,
        selection: { start: 2, end: 2 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    expect(input.value).toBe('\u{2028}e\u{2029}');

    input.setSelectionRange(1, 2);
    const beforeInput = beforeInputEvent(input, 'insertText', 'è');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).not.toHaveBeenCalled();

    input.value = '\u{2028}eè\u{2029}';
    input.setSelectionRange(3, 3);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u{2028}eè\u{2029}');
    expect(input.selectionStart).toBe(3);
    expect(input.selectionEnd).toBe(3);
    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 2, end: 2 },
          { type: 'replace_selection', text: 'è' },
        ],
      },
    ]);
  });

  it('emits a replacement when the entire flat text is replaced with the same typed text', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}a\u{2029}',
        windowStart: 0,
        selection: { start: 0, end: 3 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe(3);

    const beforeInput = beforeInputEvent(input, 'insertText', 'a');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).not.toHaveBeenCalled();

    input.value = 'a';
    input.setSelectionRange(1, 1);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 0, end: 3 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
    ]);
  });

  it('emits a replacement when selected text is replaced with the same typed text', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}a\u{2029}',
        windowStart: 0,
        selection: { start: 1, end: 2 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    expect(input.selectionStart).toBe(1);
    expect(input.selectionEnd).toBe(2);

    const beforeInput = beforeInputEvent(input, 'insertText', 'a');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).not.toHaveBeenCalled();

    input.value = '\u{2028}a\u{2029}';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 1, end: 2 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
    ]);
  });

  it('prefers the native replacement diff when beforeinput selection points at the next character', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}et\u{2029}',
        windowStart: 0,
        selection: { start: 2, end: 2 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    input.setSelectionRange(2, 3);
    const beforeInput = beforeInputEvent(input, 'insertText', 'è');
    adapter.handleBeforeInput(beforeInput);

    input.value = '\u{2028}èt\u{2029}';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u{2028}èt\u{2029}');
    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 1, end: 2 },
          { type: 'replace_selection', text: 'è' },
        ],
      },
    ]);
  });

  it('keeps a collapsed native append as an insertion after many spaces', () => {
    const input = createInput();
    const messages: Message[] = [];
    const spaces = ' '.repeat(79);
    const text = `\u{2028}${spaces}e\u{2029}`;
    const windowStart = 120;
    const eStart = 1 + spaces.length;
    const eEnd = eStart + 1;
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text,
        windowStart,
        selection: { start: windowStart + eEnd, end: windowStart + eEnd },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    input.setSelectionRange(eEnd, eEnd + 1);
    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', 'è'));

    input.value = `\u{2028}${spaces}eè\u{2029}`;
    input.setSelectionRange(eEnd + 1, eEnd + 1);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe(`\u{2028}${spaces}eè\u{2029}`);
    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: windowStart + eEnd, end: windowStart + eEnd },
          { type: 'replace_selection', text: 'è' },
        ],
      },
    ]);
  });

  it('does not turn a native collapsed insert into a synthetic previous-character replacement', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}eX\u{2029}',
        windowStart: 40,
        selection: { start: 42, end: 42 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    input.setSelectionRange(2, 3);
    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', 'è'));

    input.value = '\u{2028}eXè\u{2029}';
    input.setSelectionRange(4, 4);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 43, end: 43 },
          { type: 'replace_selection', text: 'è' },
        ],
      },
    ]);
  });

  it('maps input in an empty paragraph after previous text to the empty paragraph flat offset', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}prev\u{2029}\u{2028}\u{2029}',
        windowStart: 0,
        selection: { start: 7, end: 7 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    expect(input.value).toBe('\u{2028}prev\u{2029}\u{2028}\u{2029}');

    const beforeInput = beforeInputEvent(input, 'insertText', 'a');
    adapter.handleBeforeInput(beforeInput);

    input.value = '\u{2028}prev\u{2029}\u{2028}a\u{2029}';
    input.setSelectionRange(8, 8);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 7, end: 7 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
    ]);
  });

  it('does not suppress the next real input after a prevented line-break beforeinput', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => context(''),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    const beforeInput = beforeInputEvent(input, 'insertParagraph');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).toHaveBeenCalledOnce();

    input.value = 'a';
    input.setSelectionRange(1, 1);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      { type: 'key', event: { key: 'enter' } },
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 20, end: 20 },
          { type: 'replace_selection', text: 'a' },
        ],
      },
    ]);
  });

  it('preserves hard breaks in the hidden input when typing after a hard break', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}123\n\u{2029}',
        windowStart: 0,
        selection: { start: 5, end: 5 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);

    expect(input.value).toBe('\u{2028}123\n\u{2029}');
    expect(input.selectionStart).toBe(5);
    expect(input.selectionEnd).toBe(5);

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', '1'));
    input.value = '\u{2028}123\n1\u{2029}';
    input.setSelectionRange(6, 6);
    adapter.handleInput(inputEvent(input));

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 5, end: 5 },
          { type: 'replace_selection', text: '1' },
        ],
      },
    ]);
  });

  it('lets browser composition mutate the DOM and converts repeated updates to flat compose diffs', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => context(''),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.handleCompositionStart(compositionEvent(input));

    const first = beforeInputEvent(input, 'insertCompositionText', 'か');
    adapter.handleBeforeInput(first);
    expect(first.preventDefault).not.toHaveBeenCalled();

    input.value = 'か';
    input.setSelectionRange(1, 1);
    adapter.handleInput(inputEvent(input));
    expect(input.value).toBe('か');

    adapter.syncFromEditor(input);
    expect(input.value).toBe('か');

    const second = beforeInputEvent(input, 'insertCompositionText', 'かな');
    adapter.handleBeforeInput(second);
    expect(second.preventDefault).not.toHaveBeenCalled();

    input.value = 'かな';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));
    expect(input.value).toBe('かな');

    adapter.handleCompositionEnd();

    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 20, end: 20 },
          { type: 'compose', text: 'か' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 20, end: 21 },
          { type: 'compose', text: 'かな' },
        ],
      },
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
    ]);
  });

  it('uses insertCompositionText data instead of duplicated native DOM preedit text', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}\u{2029}',
        windowStart: 0,
        selection: { start: 1, end: 1 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    adapter.handleCompositionStart(compositionEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅎ'));
    input.value = '\u{2028}ㅎ\u{2029}';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', '하'));
    input.value = '\u{2028}ㅎ하\u{2029}';
    input.setSelectionRange(3, 3);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u{2028}하\u{2029}');
    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 1 },
          { type: 'compose', text: 'ㅎ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: '하' },
        ],
      },
    ]);
  });

  it('keeps repeated Korean preedit text growing after replacing selected text', () => {
    const input = createInput();
    const messages: Message[] = [];
    const adapter = new ImeInputAdapter({
      readContext: () => ({
        text: '\u{2028}ㅁㅁㅁ\u{2029}',
        windowStart: 0,
        selection: { start: 1, end: 4 },
        composing: null,
      }),
      enqueue: (next) => {
        messages.push(...next);
      },
    });

    adapter.syncFromEditor(input);
    adapter.handleCompositionStart(compositionEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅁ'));
    input.value = '\u{2028}ㅁ\u{2029}';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅁ'));
    input.value = '\u{2028}ㅁㅁ\u{2029}';
    input.setSelectionRange(3, 3);
    adapter.handleInput(inputEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅁ'));
    input.value = '\u{2028}ㅁㅁㅁ\u{2029}';
    input.setSelectionRange(4, 4);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u{2028}ㅁㅁㅁ\u{2029}');
    expect(messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 4 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: 'ㅁㅁ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 3 },
          { type: 'compose', text: 'ㅁㅁㅁ' },
        ],
      },
    ]);
  });

  it('starts composition when selected text is replaced with the same Korean preedit', () => {
    const ime = createImeHarness({
      text: '\u{2028}ㅁ\u{2029}',
      windowStart: 0,
      selection: { start: 1, end: 2 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('\u{2028}ㅁ\u{2029}', 2);

    ime.expectInput('\u{2028}ㅁ\u{2029}', 2);
    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
    ]);
  });

  it('keeps appended Korean preedit text in composition after replacing the full IME buffer', () => {
    const ime = createImeHarness({
      text: '\u{2028}ㅁㄴ\u{2029}',
      windowStart: 0,
      selection: { start: 0, end: 4 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u{2028}ㅁ\u{2029}',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: { start: 1, end: 2 },
    });
    ime.syncFromEditor();

    ime.expectInput('ㅁ', 1);

    const duplicateCommittedPreedit = ime.beforeTextInput('ㅁ');
    expect(duplicateCommittedPreedit.preventDefault).toHaveBeenCalledOnce();
    expect(ime.messages).toHaveLength(1);

    ime.beforeCompositionInput('ㄴ');
    ime.applyNativeInput('ㅁㄴ', 2);

    ime.expectInput('ㅁㄴ', 2);

    ime.setContext({
      text: '\u{2028}ㅁㄴ\u{2029}',
      windowStart: 0,
      selection: { start: 3, end: 3 },
      composing: { start: 2, end: 3 },
    });
    ime.syncFromEditor();

    ime.compositionStart();
    ime.compositionUpdate('ㄴㅇ');
    ime.beforeCompositionInput('ㄴㅇ');
    ime.applyNativeInput('ㅁㄴㅇ', 3);

    ime.expectInput('ㅁㄴㅇ', 3);
    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 0, end: 4 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 2, end: 2 },
          { type: 'compose', text: 'ㄴ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 2, end: 3 },
          { type: 'compose', text: 'ㄴㅇ' },
        ],
      },
    ]);
  });

  it('preserves the native buffer while rebasing an editor-restored structure during composition', () => {
    const ime = createImeHarness({
      text: '\u{2028}\u{2028}\u{2029}\u{2029}',
      windowStart: 0,
      selection: { start: 0, end: 4 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u{2028}\u{2028}\u{2029}\u{2029}\u{2028}ㅁ\u{2029}',
      windowStart: 0,
      selection: { start: 6, end: 6 },
      composing: { start: 5, end: 6 },
    });
    ime.syncFromEditor();

    ime.expectInput('ㅁ', 1);

    ime.compositionStart();
    ime.compositionUpdate('ㅁㄴ');
    ime.beforeCompositionInput('ㅁㄴ');
    ime.applyNativeInput('ㅁㄴ', 2);

    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 0, end: 4 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 5, end: 6 },
          { type: 'compose', text: 'ㅁㄴ' },
        ],
      },
    ]);
  });

  it('infers composition when editor-restored structure omits it during a full-buffer replacement', () => {
    const ime = createImeHarness({
      text: '\u{2028}ㅁㄴㅇ\u{2029}',
      windowStart: 0,
      selection: { start: 0, end: 5 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart('\u{2028}ㅁㄴㅇ\u{2029}');
    ime.compositionUpdate('ㅁ');
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u{2028}ㅁ\u{2029}',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: null,
    });
    ime.syncFromEditor();

    ime.expectInput('ㅁ', 1);

    ime.compositionUpdate('ㅁ');
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);
    ime.compositionEnd();

    ime.compositionStart();
    ime.compositionUpdate('ㄴ');
    ime.beforeCompositionInput('ㄴ');
    ime.applyNativeInput('ㅁㄴ', 2);

    ime.expectInput('ㅁㄴ', 2);

    ime.setContext({
      text: '\u{2028}ㅁㄴ\u{2029}',
      windowStart: 0,
      selection: { start: 3, end: 3 },
      composing: null,
    });
    ime.syncFromEditor();

    const duplicateSecondPreedit = ime.beforeTextInput('ㄴ');
    expect(duplicateSecondPreedit.preventDefault).toHaveBeenCalledOnce();

    ime.compositionStart();
    ime.compositionUpdate('ㅇ');
    ime.beforeCompositionInput('ㅇ');
    ime.applyNativeInput('ㅁㄴㅇ', 3);

    ime.expectInput('ㅁㄴㅇ', 3);
    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 0, end: 5 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 2, end: 2 },
          { type: 'compose', text: 'ㄴ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 3, end: 3 },
          { type: 'compose', text: 'ㅇ' },
        ],
      },
    ]);
  });

  it('ignores duplicate committed preedit after replacing a selected empty paragraph', () => {
    const ime = createImeHarness({
      text: '\u{2028}\u{2029}',
      windowStart: 0,
      selection: { start: 0, end: 2 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart('\u{2028}\u{2029}');
    ime.compositionUpdate('ㅁ');
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u{2028}ㅁ\u{2029}',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: { start: 1, end: 2 },
    });
    ime.syncFromEditor();

    const duplicateCommittedPreedit = ime.beforeTextInput('ㅁ');
    expect(duplicateCommittedPreedit.preventDefault).toHaveBeenCalledOnce();

    ime.compositionStart();
    ime.compositionUpdate('ㄴ');
    ime.beforeCompositionInput('ㄴ');
    ime.applyNativeInput('ㅁㄴ', 2);

    ime.setContext({
      text: '\u{2028}ㅁㄴ\u{2029}',
      windowStart: 0,
      selection: { start: 3, end: 3 },
      composing: { start: 2, end: 3 },
    });
    ime.syncFromEditor();

    ime.compositionEnd();
    ime.compositionStart();
    ime.compositionUpdate('ㅇ');
    ime.beforeCompositionInput('ㅇ');
    ime.applyNativeInput('ㅁㄴㅇ', 3);

    ime.expectInput('ㅁㄴㅇ', 3);
    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 0, end: 2 },
          { type: 'compose', text: 'ㅁ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 2, end: 2 },
          { type: 'compose', text: 'ㄴ' },
        ],
      },
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 3, end: 3 },
          { type: 'compose', text: 'ㅇ' },
        ],
      },
    ]);
  });

  it('preserves the native buffer while rebasing a selected empty paragraph during composition', () => {
    const ime = createImeHarness({
      text: '\u{2028}\u{2029}',
      windowStart: 0,
      selection: { start: 0, end: 2 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart('\u{2028}\u{2029}');
    ime.compositionUpdate('n');
    ime.beforeCompositionInput('n');
    ime.applyNativeInput('n', 1);

    ime.setContext({
      text: '\u{2028}n\u{2029}',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: { start: 1, end: 2 },
    });
    ime.syncFromEditor();

    ime.expectInput('n', 1);

    ime.compositionStart();
    ime.compositionUpdate('に');
    ime.beforeCompositionInput('に');
    ime.applyNativeInput('に', 1);

    ime.expectInput('に', 1);
    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 0, end: 2 },
          { type: 'compose', text: 'n' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: 'に' },
        ],
      },
    ]);
  });

  it('keeps a romanized IME composition in one native buffer after selected empty paragraph replacement', () => {
    const ime = createImeHarness({
      text: '\u{2028}\u{2029}',
      windowStart: 0,
      selection: { start: 0, end: 2 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart('\u{2028}\u{2029}');
    ime.compositionUpdate('n');
    ime.beforeCompositionInput('n');
    ime.applyNativeInput('n', 1);

    ime.setContext({
      text: '\u{2028}n\u{2029}',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: { start: 1, end: 2 },
    });
    ime.syncFromEditor();

    for (const [text, cursor] of [
      ['に', 1],
      ['にh', 2],
      ['にほ', 2],
      ['にほn', 3],
      ['にほんg', 4],
      ['日本語', 3],
    ] as const) {
      ime.compositionUpdate(text);
      ime.beforeCompositionInput(text);
      ime.applyNativeInput(text, cursor);
    }

    ime.expectInput('日本語', 3);
    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 0, end: 2 },
          { type: 'compose', text: 'n' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: 'に' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 2 },
          { type: 'compose', text: 'にh' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 3 },
          { type: 'compose', text: 'にほ' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 3 },
          { type: 'compose', text: 'にほn' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 4 },
          { type: 'compose', text: 'にほんg' },
        ],
      },
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 1, end: 5 },
          { type: 'compose', text: '日本語' },
        ],
      },
    ]);
  });

  it('inserts the space redispatched right after a space-committed Korean composition (Windows IME timing)', () => {
    const ime = createImeHarness(context(''));

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('한');
    ime.applyNativeInput('한', 1);
    ime.compositionEnd();

    const spaceEvent = ime.beforeTextInput(' ');
    expect(spaceEvent.preventDefault).not.toHaveBeenCalled();

    ime.applyNativeInput('한 ', 2);

    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 20, end: 20 },
          { type: 'compose', text: '한' },
        ],
      },
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
      {
        type: 'text_input',
        ops: [
          { type: 'set_selection', start: 21, end: 21 },
          { type: 'replace_selection', text: ' ' },
        ],
      },
    ]);
  });

  it('ignores a duplicate committed preedit insertText right after compositionend', () => {
    const ime = createImeHarness(context(''));

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('한');
    ime.applyNativeInput('한', 1);
    ime.compositionEnd();

    const duplicateEvent = ime.beforeTextInput('한');
    expect(duplicateEvent.preventDefault).toHaveBeenCalled();

    expect(ime.messages).toEqual([
      {
        type: 'text_input',
        ops: [
          { type: 'set_composition', start: 20, end: 20 },
          { type: 'compose', text: '한' },
        ],
      },
      { type: 'text_input', ops: [{ type: 'commit_as_is' }] },
    ]);
  });

  it('resetForResync rewrites the input from editor state before cycling focus', () => {
    const harness = createImeHarness(context(''));
    const { adapter, input } = harness;
    document.body.append(input);

    adapter.handleCompositionStart(compositionEvent(input));
    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅎ'));
    input.value = 'ㅎ';
    input.setSelectionRange(1, 1);
    adapter.handleInput(inputEvent(input));

    input.focus();
    expect(document.activeElement).toBe(input);

    harness.setContext({
      text: '\u{2028}done\u{2029}',
      windowStart: 0,
      selection: { start: 5, end: 5 },
      composing: null,
    });

    const order: string[] = [];
    let valueWhenBlurred: string | null = null;
    vi.spyOn(input, 'blur').mockImplementation(() => {
      valueWhenBlurred = input.value;
      order.push('blur');
    });
    vi.spyOn(input, 'focus').mockImplementation(() => {
      order.push('focus');
    });

    adapter.resetForResync(input);

    expect(input.value).toBe('\u{2028}done\u{2029}');
    expect(input.selectionStart).toBe(5);
    expect(valueWhenBlurred).toBe('\u{2028}done\u{2029}');
    expect(order).toEqual(['blur', 'focus']);
  });

  it('resetForResync outside composition rewrites without focus cycling', () => {
    const harness = createImeHarness(context(''));
    const { adapter, input } = harness;
    document.body.append(input);

    adapter.syncFromEditor(input);

    harness.setContext({
      text: '\u{2028}reset\u{2029}',
      windowStart: 0,
      selection: { start: 6, end: 6 },
      composing: null,
    });

    const blur = vi.spyOn(input, 'blur');
    const focus = vi.spyOn(input, 'focus');

    adapter.resetForResync(input);

    expect(input.value).toBe('\u{2028}reset\u{2029}');
    expect(input.selectionStart).toBe(6);
    expect(input.selectionEnd).toBe(6);
    expect(blur).not.toHaveBeenCalled();
    expect(focus).not.toHaveBeenCalled();
  });

  it('input and composition events during resync are dropped', () => {
    const harness = createImeHarness(context(''));
    const { adapter, input, messages } = harness;
    document.body.append(input);

    adapter.handleCompositionStart(compositionEvent(input));
    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅎ'));
    input.value = 'ㅎ';
    input.setSelectionRange(1, 1);
    adapter.handleInput(inputEvent(input));

    input.focus();
    expect(document.activeElement).toBe(input);

    harness.setContext({
      text: '\u{2028}sync\u{2029}',
      windowStart: 0,
      selection: { start: 5, end: 5 },
      composing: null,
    });

    vi.spyOn(input, 'blur').mockImplementation(() => {
      input.value = 'garbage';
      input.setSelectionRange(7, 7);
      adapter.handleInput(inputEvent(input));
      adapter.handleCompositionEnd();
    });

    messages.length = 0;

    adapter.resetForResync(input);

    expect(messages).toEqual([]);
  });
});
