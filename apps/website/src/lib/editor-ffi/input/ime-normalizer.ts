import { codePointLength, codePoints, codePointSlice } from './ime-context';
import type { FlatImeOp, Message } from '@typie/editor-ffi/browser';
import type { ImeContext } from './ime-context';

export type DomInputDiff = {
  start: number;
  end: number;
  insertedText: string;
};

export type DomComposingReplacement = {
  targetStart: number;
  targetEnd: number;
  nextStart: number;
  nextEnd: number;
  text: string;
};

export const flatImeMessage = (ops: FlatImeOp[]): Message[] => {
  if (ops.length === 0) {
    return [];
  }

  return [{ type: 'composition', op: { type: 'flat', ops } }];
};

export const readDomInputDiff = (context: ImeContext, nextText: string): DomInputDiff | null => {
  if (context.text === nextText) {
    return null;
  }

  const prevChars = codePoints(context.text);
  const nextChars = codePoints(nextText);

  let prefix = 0;
  while (prefix < prevChars.length && prefix < nextChars.length && prevChars[prefix] === nextChars[prefix]) {
    prefix += 1;
  }

  let prevSuffix = prevChars.length;
  let nextSuffix = nextChars.length;
  while (prevSuffix > prefix && nextSuffix > prefix && prevChars[prevSuffix - 1] === nextChars[nextSuffix - 1]) {
    prevSuffix -= 1;
    nextSuffix -= 1;
  }

  const start = context.windowStart + prefix;
  const end = context.windowStart + prevSuffix;
  const insertedText = nextChars.slice(prefix, nextSuffix).join('');
  return { start, end, insertedText };
};

export const readDomComposingReplacement = (context: ImeContext, nextText: string, diff: DomInputDiff): DomComposingReplacement => {
  const previous = context.composing ?? { start: diff.start, end: diff.end };
  const removedLength = diff.end - diff.start;
  const insertedLength = codePointLength(diff.insertedText);
  const nextStart = previous.start;
  const nextEnd = Math.max(previous.start, previous.end + insertedLength - removedLength);
  const localStart = Math.max(0, nextStart - context.windowStart);
  const localEnd = Math.max(localStart, nextEnd - context.windowStart);

  return {
    targetStart: previous.start,
    targetEnd: previous.end,
    nextStart,
    nextEnd,
    text: codePointSlice(nextText, localStart, localEnd),
  };
};

export const normalizeLineBreakBeforeInput = (inputType: string): Message[] => {
  if (inputType !== 'insertLineBreak' && inputType !== 'insertParagraph') {
    return [];
  }

  return [{ type: 'key', event: { key: 'enter' } }];
};
