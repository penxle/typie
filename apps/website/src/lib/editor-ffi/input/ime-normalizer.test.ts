import { describe, expect, it } from 'vitest';
import { normalizeLineBreakBeforeInput, readDomComposingReplacement, readDomInputDiff } from './ime-normalizer';
import { composingContext, context } from './ime-test-fixtures';
import type { ImeContext } from './ime-context';

describe('web IME normalizer', () => {
  it('reads a plain native input mutation as a DOM text diff', () => {
    expect(readDomInputDiff(context(''), 'a')).toEqual({
      start: 20,
      end: 20,
      insertedText: 'a',
    });
  });

  it('reads macOS accent-popup replacement as a DOM replacement diff', () => {
    expect(
      readDomInputDiff(
        {
          text: '\u{2028}e\u{2029}',
          windowStart: 0,
          selection: { start: 2, end: 2 },
          composing: null,
        },
        '\u{2028}è\u{2029}',
      ),
    ).toEqual({
      start: 1,
      end: 2,
      insertedText: 'è',
    });
  });

  it('reads insertion into an empty token-only paragraph at the inner flat offset', () => {
    expect(
      readDomInputDiff(
        {
          text: '\u{2028}\u{2029}',
          windowStart: 0,
          selection: { start: 1, end: 1 },
          composing: null,
        },
        '\u{2028}a\u{2029}',
      ),
    ).toEqual({
      start: 1,
      end: 1,
      insertedText: 'a',
    });
  });

  it('reads deletion by native input diff instead of synthesizing a key event', () => {
    expect(readDomInputDiff(context('abc'), 'ac')).toEqual({
      start: 21,
      end: 22,
      insertedText: '',
    });
  });

  it('keeps surrogate-pair replacement on flat code-point boundaries', () => {
    expect(readDomInputDiff(context('a😀b'), 'aXb')).toEqual({
      start: 21,
      end: 22,
      insertedText: 'X',
    });
  });

  it('does not produce a DOM diff for a selection-only change', () => {
    expect(readDomInputDiff(context('abc'), 'abc')).toBeNull();
  });

  it('reads native composition replacement from the active composing range', () => {
    const current = composingContext('か', 20, 21);
    const nextText = 'かな';
    const diff = readDomInputDiff(current, nextText);

    expect(diff).toEqual({ start: 21, end: 21, insertedText: 'な' });
    if (!diff) throw new Error('expected DOM input diff');
    expect(readDomComposingReplacement(current, nextText, diff)).toEqual({
      targetStart: 20,
      targetEnd: 21,
      nextStart: 20,
      nextEnd: 22,
      text: 'かな',
    });
  });

  it('reads composition inside an empty token-only paragraph at inner flat offsets', () => {
    const current: ImeContext = {
      text: '\u{2028}\u{2029}',
      windowStart: 0,
      selection: { start: 1, end: 1 },
      composing: { start: 1, end: 1 },
    };
    const nextText = '\u{2028}ㅎ\u{2029}';
    const diff = readDomInputDiff(current, nextText);

    expect(diff).toEqual({ start: 1, end: 1, insertedText: 'ㅎ' });
    if (!diff) throw new Error('expected DOM input diff');
    expect(readDomComposingReplacement(current, nextText, diff)).toEqual({
      targetStart: 1,
      targetEnd: 1,
      nextStart: 1,
      nextEnd: 2,
      text: 'ㅎ',
    });
  });

  it('preserves line-break beforeinput as the editor enter key path', () => {
    expect(normalizeLineBreakBeforeInput('insertParagraph')).toEqual([{ type: 'key', event: { key: 'enter' } }]);
    expect(normalizeLineBreakBeforeInput('insertLineBreak')).toEqual([{ type: 'key', event: { key: 'enter' } }]);
    expect(normalizeLineBreakBeforeInput('insertText')).toEqual([]);
  });
});
