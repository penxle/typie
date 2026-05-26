import { describe, expect, it } from 'vitest';
import {
  flatOffsetToUtf16Index,
  normalizeImeContext,
  readInputUtf16Selection,
  syncInputElementToContext,
  updateContextFromInputElement,
  utf16IndexToFlatOffset,
  utf16SelectionToFlatRange,
} from './ime-context';
import { context } from './ime-test-fixtures';
import type { Ime } from '@typie/editor-ffi/browser';

describe('web IME context', () => {
  it('normalizes the FFI IME payload into web adapter shape', () => {
    const ime: Ime = {
      text: 'hello',
      window_start: 10,
      selection: { start: 12, end: 14 },
      composing: undefined,
    };

    expect(normalizeImeContext(ime)).toEqual({
      text: 'hello',
      windowStart: 10,
      selection: { start: 12, end: 14 },
      composing: null,
    });
  });

  it('keeps structural flat tokens in the hidden input context', () => {
    const ime: Ime = {
      text: '\u2028e\u2029',
      window_start: 0,
      selection: { start: 2, end: 2 },
      composing: undefined,
    };

    expect(normalizeImeContext(ime)).toEqual({
      text: '\u2028e\u2029',
      windowStart: 0,
      selection: { start: 2, end: 2 },
      composing: null,
    });
  });

  it('keeps an insertion point inside an empty flat token window', () => {
    const ime: Ime = {
      text: '\u2028\u2029',
      window_start: 0,
      selection: { start: 1, end: 1 },
      composing: undefined,
    };

    expect(normalizeImeContext(ime)).toEqual({
      text: '\u2028\u2029',
      windowStart: 0,
      selection: { start: 1, end: 1 },
      composing: null,
    });
  });

  it('maps flat offsets to DOM UTF-16 indices across surrogate pairs', () => {
    const text = 'a😀b';
    const windowStart = 20;

    expect(flatOffsetToUtf16Index(text, windowStart, 20)).toBe(0);
    expect(flatOffsetToUtf16Index(text, windowStart, 21)).toBe(1);
    expect(flatOffsetToUtf16Index(text, windowStart, 22)).toBe(3);
    expect(flatOffsetToUtf16Index(text, windowStart, 23)).toBe(4);
  });

  it('maps DOM UTF-16 indices back to flat offsets across surrogate pairs', () => {
    const text = 'a😀b';
    const windowStart = 20;

    expect(utf16IndexToFlatOffset(text, windowStart, 0)).toBe(20);
    expect(utf16IndexToFlatOffset(text, windowStart, 1)).toBe(21);
    expect(utf16IndexToFlatOffset(text, windowStart, 3)).toBe(22);
    expect(utf16IndexToFlatOffset(text, windowStart, 4)).toBe(23);
  });

  it('syncs the hidden input value and DOM selection from flat offsets', () => {
    const input = document.createElement('input');

    syncInputElementToContext(input, {
      text: 'a😀b',
      windowStart: 20,
      selection: { start: 21, end: 22 },
      composing: null,
    });

    expect(input.value).toBe('a😀b');
    expect(input.selectionStart).toBe(1);
    expect(input.selectionEnd).toBe(3);
  });

  it('syncs an empty paragraph cursor without collapsing it to previous text', () => {
    const input = document.createElement('input');

    syncInputElementToContext(input, {
      text: '\u2028prev\u2029\u2028\u2029',
      windowStart: 0,
      selection: { start: 7, end: 7 },
      composing: null,
    });

    expect(input.value).toBe('\u2028prev\u2029\u2028\u2029');
    expect(input.selectionStart).toBe(7);
    expect(input.selectionEnd).toBe(7);
  });

  it('reads DOM selection as absolute flat offsets', () => {
    const input = document.createElement('input');
    input.value = 'a😀b';
    input.setSelectionRange(1, 3);

    expect(readInputUtf16Selection(input)).toEqual({ start: 1, end: 3 });
    expect(utf16SelectionToFlatRange('a😀b', 20, readInputUtf16Selection(input))).toEqual({ start: 21, end: 22 });
  });

  it('updates local context from DOM input and preserves the supplied composing range', () => {
    const input = document.createElement('input');
    input.value = 'aXb';
    input.setSelectionRange(2, 2);

    expect(
      updateContextFromInputElement(
        {
          ...context('a😀b'),
          selection: { start: 20, end: 20 },
        },
        input,
        { start: 21, end: 22 },
      ),
    ).toEqual({
      text: 'aXb',
      windowStart: 20,
      selection: { start: 22, end: 22 },
      composing: { start: 21, end: 22 },
    });
  });
});
