import { clamp } from '@typie/ui/utils';
import type { Ime } from '@typie/editor-ffi/browser';

export type ImeTextInput = HTMLTextAreaElement;

export const IME_CONTEXT_BEFORE_LIMIT = 64;
export const IME_CONTEXT_AFTER_LIMIT = 64;

export type ImeRange = {
  start: number;
  end: number;
};

export type Utf16Selection = {
  start: number;
  end: number;
};

export type ImeContext = {
  text: string;
  windowStart: number;
  selection: ImeRange;
  composing: ImeRange | null;
};

export const codePoints = (text: string): string[] => [...text];

export const codePointLength = (text: string): number => codePoints(text).length;

export const codePointSlice = (text: string, start: number, end: number): string => codePoints(text).slice(start, end).join('');

export const normalizeImeContext = (ime: Ime): ImeContext => ({
  text: ime.text,
  windowStart: ime.window_start,
  selection: { start: ime.selection.start, end: ime.selection.end },
  composing: ime.composing ? { start: ime.composing.start, end: ime.composing.end } : null,
});

export const flatOffsetToUtf16Index = (text: string, windowStart: number, flatOffset: number): number => {
  const localOffset = clamp(flatOffset - windowStart, 0, codePointLength(text));
  let codePointOffset = 0;
  let utf16Index = 0;

  for (const char of text) {
    if (codePointOffset === localOffset) {
      break;
    }

    codePointOffset += 1;
    utf16Index += char.length;
  }

  return utf16Index;
};

export const utf16IndexToFlatOffset = (text: string, windowStart: number, utf16Index: number): number => {
  const target = clamp(utf16Index, 0, text.length);
  let flatOffset = windowStart;
  let currentUtf16Index = 0;

  for (const char of text) {
    const nextUtf16Index = currentUtf16Index + char.length;
    if (nextUtf16Index > target) {
      break;
    }

    flatOffset += 1;
    currentUtf16Index = nextUtf16Index;
  }

  return flatOffset;
};

export const readInputUtf16Selection = (input: ImeTextInput): Utf16Selection => {
  const start = input.selectionStart ?? 0;
  return {
    start,
    end: input.selectionEnd ?? start,
  };
};

export const utf16SelectionToFlatRange = (text: string, windowStart: number, selection: Utf16Selection): ImeRange => {
  const anchor = utf16IndexToFlatOffset(text, windowStart, selection.start);
  const head = utf16IndexToFlatOffset(text, windowStart, selection.end);
  return {
    start: Math.min(anchor, head),
    end: Math.max(anchor, head),
  };
};

export const syncInputElementToContext = (input: ImeTextInput, context: ImeContext): void => {
  if (input.value !== context.text) {
    input.value = context.text;
  }

  const selectionStart = flatOffsetToUtf16Index(context.text, context.windowStart, context.selection.start);
  const selectionEnd = flatOffsetToUtf16Index(context.text, context.windowStart, context.selection.end);
  input.setSelectionRange(selectionStart, selectionEnd);
};

export const replaceContextRange = (context: ImeContext, range: ImeRange, text: string): string => {
  const chars = [...context.text];
  const start = clamp(range.start - context.windowStart, 0, chars.length);
  const end = clamp(range.end - context.windowStart, 0, chars.length);
  return [...chars.slice(0, start), text, ...chars.slice(end)].join('');
};

export const rangesEqual = (left: ImeRange | null, right: ImeRange | null): boolean =>
  left === right || (left != null && right != null && left.start === right.start && left.end === right.end);

export const contextWindowsOverlapEqual = (left: ImeContext, right: ImeContext): boolean => {
  const leftEnd = left.windowStart + codePointLength(left.text);
  const rightEnd = right.windowStart + codePointLength(right.text);
  const overlapStart = Math.max(left.windowStart, right.windowStart);
  const overlapEnd = Math.min(leftEnd, rightEnd);
  if (overlapStart >= overlapEnd) {
    return false;
  }

  return (
    codePointSlice(left.text, overlapStart - left.windowStart, overlapEnd - left.windowStart) ===
    codePointSlice(right.text, overlapStart - right.windowStart, overlapEnd - right.windowStart)
  );
};

export const canPreserveNativeInputOnEditorSync = (local: ImeContext, incoming: ImeContext): boolean =>
  rangesEqual(local.selection, incoming.selection) &&
  rangesEqual(local.composing, incoming.composing) &&
  contextWindowsOverlapEqual(local, incoming);

export const updateContextFromInputElement = (context: ImeContext, input: ImeTextInput, composing: ImeRange | null = null): ImeContext => {
  const text = input.value;
  const selection = utf16SelectionToFlatRange(text, context.windowStart, readInputUtf16Selection(input));
  return {
    ...context,
    text,
    selection,
    composing,
  };
};
