// cspell:ignore aeeeeeb aeeeeeeb xeeeeeb eeeeeeb xeeeeeeb

import { describe, expect, it } from 'vitest';
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
    enqueue: (next) => messages.push(...next),
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
      enqueue: (next) => messages.push(...next),
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
      enqueue: (next) => messages.push(...next),
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
      enqueue: (next) => messages.push(...next),
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
        text: '\u2028e\u2029',
        windowStart: 0,
        selection: { start: 2, end: 2 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    expect(input.value).toBe('\u2028e\u2029');

    input.setSelectionRange(1, 2);
    const beforeInput = beforeInputEvent(input, 'insertText', 'è');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).not.toHaveBeenCalled();

    input.value = '\u2028eè\u2029';
    input.setSelectionRange(3, 3);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u2028eè\u2029');
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
        text: '\u2028a\u2029',
        windowStart: 0,
        selection: { start: 0, end: 3 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
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
        text: '\u2028a\u2029',
        windowStart: 0,
        selection: { start: 1, end: 2 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    expect(input.selectionStart).toBe(1);
    expect(input.selectionEnd).toBe(2);

    const beforeInput = beforeInputEvent(input, 'insertText', 'a');
    adapter.handleBeforeInput(beforeInput);
    expect(beforeInput.preventDefault).not.toHaveBeenCalled();

    input.value = '\u2028a\u2029';
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
        text: '\u2028et\u2029',
        windowStart: 0,
        selection: { start: 2, end: 2 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    input.setSelectionRange(2, 3);
    const beforeInput = beforeInputEvent(input, 'insertText', 'è');
    adapter.handleBeforeInput(beforeInput);

    input.value = '\u2028èt\u2029';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u2028èt\u2029');
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
    const text = `\u2028${spaces}e\u2029`;
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
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    input.setSelectionRange(eEnd, eEnd + 1);
    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', 'è'));

    input.value = `\u2028${spaces}eè\u2029`;
    input.setSelectionRange(eEnd + 1, eEnd + 1);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe(`\u2028${spaces}eè\u2029`);
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
        text: '\u2028eX\u2029',
        windowStart: 40,
        selection: { start: 42, end: 42 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    input.setSelectionRange(2, 3);
    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', 'è'));

    input.value = '\u2028eXè\u2029';
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
        text: '\u2028prev\u2029\u2028\u2029',
        windowStart: 0,
        selection: { start: 7, end: 7 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    expect(input.value).toBe('\u2028prev\u2029\u2028\u2029');

    const beforeInput = beforeInputEvent(input, 'insertText', 'a');
    adapter.handleBeforeInput(beforeInput);

    input.value = '\u2028prev\u2029\u2028a\u2029';
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
      enqueue: (next) => messages.push(...next),
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
        text: '\u2028123\n\u2029',
        windowStart: 0,
        selection: { start: 5, end: 5 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);

    expect(input.value).toBe('\u2028123\n\u2029');
    expect(input.selectionStart).toBe(5);
    expect(input.selectionEnd).toBe(5);

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertText', '1'));
    input.value = '\u2028123\n1\u2029';
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
      enqueue: (next) => messages.push(...next),
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
        text: '\u2028\u2029',
        windowStart: 0,
        selection: { start: 1, end: 1 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    adapter.handleCompositionStart(compositionEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅎ'));
    input.value = '\u2028ㅎ\u2029';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', '하'));
    input.value = '\u2028ㅎ하\u2029';
    input.setSelectionRange(3, 3);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u2028하\u2029');
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
        text: '\u2028ㅁㅁㅁ\u2029',
        windowStart: 0,
        selection: { start: 1, end: 4 },
        composing: null,
      }),
      enqueue: (next) => messages.push(...next),
    });

    adapter.syncFromEditor(input);
    adapter.handleCompositionStart(compositionEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅁ'));
    input.value = '\u2028ㅁ\u2029';
    input.setSelectionRange(2, 2);
    adapter.handleInput(inputEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅁ'));
    input.value = '\u2028ㅁㅁ\u2029';
    input.setSelectionRange(3, 3);
    adapter.handleInput(inputEvent(input));

    adapter.handleBeforeInput(beforeInputEvent(input, 'insertCompositionText', 'ㅁ'));
    input.value = '\u2028ㅁㅁㅁ\u2029';
    input.setSelectionRange(4, 4);
    adapter.handleInput(inputEvent(input));

    expect(input.value).toBe('\u2028ㅁㅁㅁ\u2029');
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
      text: '\u2028ㅁ\u2029',
      windowStart: 0,
      selection: { start: 1, end: 2 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('\u2028ㅁ\u2029', 2);

    ime.expectInput('\u2028ㅁ\u2029', 2);
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
      text: '\u2028ㅁㄴ\u2029',
      windowStart: 0,
      selection: { start: 0, end: 4 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u2028ㅁ\u2029',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: { start: 1, end: 2 },
    });
    ime.syncFromEditor();

    ime.expectInput('\u2028ㅁ\u2029', 2);

    const duplicateCommittedPreedit = ime.beforeTextInput('ㅁ');
    expect(duplicateCommittedPreedit.preventDefault).toHaveBeenCalledOnce();
    expect(ime.messages).toHaveLength(1);

    ime.beforeCompositionInput('ㄴ');
    ime.applyNativeInput('\u2028ㅁㄴ\u2029', 3);

    ime.expectInput('\u2028ㅁㄴ\u2029', 3);

    ime.setContext({
      text: '\u2028ㅁㄴ\u2029',
      windowStart: 0,
      selection: { start: 3, end: 3 },
      composing: { start: 2, end: 3 },
    });
    ime.syncFromEditor();

    ime.compositionStart();
    ime.beforeCompositionInput('ㅇ');
    ime.applyNativeInput('\u2028ㅁㄴㅇ\u2029', 4);

    ime.expectInput('\u2028ㅁㄴㅇ\u2029', 4);
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
          { type: 'set_composition', start: 3, end: 3 },
          { type: 'compose', text: 'ㅇ' },
        ],
      },
    ]);
  });

  it('syncs editor-restored structure while composing after replacing a structural selection', () => {
    const ime = createImeHarness({
      text: '\u2028\u2028\u2029\u2029',
      windowStart: 0,
      selection: { start: 0, end: 4 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart();
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u2028\u2028\u2029\u2029\u2028ㅁ\u2029',
      windowStart: 0,
      selection: { start: 6, end: 6 },
      composing: { start: 5, end: 6 },
    });
    ime.syncFromEditor();

    ime.expectInput('\u2028\u2028\u2029\u2029\u2028ㅁ\u2029', 6);

    ime.compositionStart();
    ime.beforeCompositionInput('ㄴ');
    ime.applyNativeInput('\u2028\u2028\u2029\u2029\u2028ㅁㄴ\u2029', 7);

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
          { type: 'set_composition', start: 6, end: 6 },
          { type: 'compose', text: 'ㄴ' },
        ],
      },
    ]);
  });

  it('infers composition when editor-restored structure omits it during a full-buffer replacement', () => {
    const ime = createImeHarness({
      text: '\u2028ㅁㄴㅇ\u2029',
      windowStart: 0,
      selection: { start: 0, end: 5 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart('\u2028ㅁㄴㅇ\u2029');
    ime.compositionUpdate('ㅁ');
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u2028ㅁ\u2029',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: null,
    });
    ime.syncFromEditor();

    ime.expectInput('\u2028ㅁ\u2029', 2);

    ime.compositionUpdate('ㅁ');
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('\u2028ㅁ\u2029', 2);
    ime.compositionEnd();

    ime.compositionStart();
    ime.compositionUpdate('ㄴ');
    ime.beforeCompositionInput('ㄴ');
    ime.applyNativeInput('\u2028ㅁㄴ\u2029', 3);

    ime.expectInput('\u2028ㅁㄴ\u2029', 3);

    ime.setContext({
      text: '\u2028ㅁㄴ\u2029',
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
    ime.applyNativeInput('\u2028ㅁㄴㅇ\u2029', 4);

    ime.expectInput('\u2028ㅁㄴㅇ\u2029', 4);
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
      text: '\u2028\u2029',
      windowStart: 0,
      selection: { start: 0, end: 2 },
      composing: null,
    });

    ime.syncFromEditor();
    ime.compositionStart('\u2028\u2029');
    ime.compositionUpdate('ㅁ');
    ime.beforeCompositionInput('ㅁ');
    ime.applyNativeInput('ㅁ', 1);

    ime.setContext({
      text: '\u2028ㅁ\u2029',
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
    ime.applyNativeInput('\u2028ㅁㄴ\u2029', 3);

    ime.setContext({
      text: '\u2028ㅁㄴ\u2029',
      windowStart: 0,
      selection: { start: 3, end: 3 },
      composing: { start: 2, end: 3 },
    });
    ime.syncFromEditor();

    ime.compositionEnd();
    ime.compositionStart();
    ime.compositionUpdate('ㅇ');
    ime.beforeCompositionInput('ㅇ');
    ime.applyNativeInput('\u2028ㅁㄴㅇ\u2029', 4);

    ime.expectInput('\u2028ㅁㄴㅇ\u2029', 4);
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
});
