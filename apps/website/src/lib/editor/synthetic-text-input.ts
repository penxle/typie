import type { Selection } from './types';

const SENTINEL = import.meta.env.PROD ? '\u200B' : '◆';
const MARKER_BASE = 0xe0_00;
const EMPTY_COMPOSING = { start: -1, end: -1 } as const;

const graphemeSegmenter =
  typeof Intl !== 'undefined' && 'Segmenter' in Intl ? new Intl.Segmenter(undefined, { granularity: 'grapheme' }) : null;

export type TextSelection = {
  baseOffset: number;
  extentOffset: number;
};

export type TextRange = {
  start: number;
  end: number;
};

export type TextEditingValue = {
  text: string;
  selection: TextSelection;
  composing: TextRange;
};

type DeltaBase = {
  oldText: string;
  selection: TextSelection;
  composing: TextRange;
};

export type TextEditingDeltaInsertion = DeltaBase & {
  type: 'insertion';
  textInserted: string;
  insertionOffset: number;
};

export type TextEditingDeltaDeletion = DeltaBase & {
  type: 'deletion';
  deletedRange: TextRange;
};

export type TextEditingDeltaReplacement = DeltaBase & {
  type: 'replacement';
  replacedRange: TextRange;
  replacementText: string;
};

export type TextEditingDeltaNonTextUpdate = DeltaBase & {
  type: 'nonTextUpdate';
};

export type SyntheticTextEditingDelta =
  | TextEditingDeltaInsertion
  | TextEditingDeltaDeletion
  | TextEditingDeltaReplacement
  | TextEditingDeltaNonTextUpdate;

export type SyntheticInputMessage =
  | { type: 'input'; text: string }
  | { type: 'deleteBackward'; length?: number }
  | { type: 'replaceBackward'; length: number; text: string }
  | { type: 'compositionStart'; text: string }
  | { type: 'compositionUpdate'; text: string; replaceLength?: number }
  | { type: 'commitPreedit' }
  | { type: 'compositionEnd' }
  | { type: 'insertNewline' }
  | { type: 'navigate'; direction: string; extend: false };

export type SyntheticTextInputState = {
  currentValue: TextEditingValue;
  sentinelLost: boolean;
  hadDeltaSinceReconcile: boolean;
  extendedComposingStart: number | null;
  reconcileNodeId: string | null;
  reconcileCursorOffset: number | null;
};

export type PendingDeltaState = {
  inputType: string | null;
  data: string | null;
  deltaText: string;
  deltaStart: number;
  deltaEnd: number;
};

export type CompositionTrackingState = {
  active: boolean;
  text: string | null;
  base: number | null;
  replaceStart: number | null;
  replaceEnd: number | null;
};

export type ReduceEditingDeltasResult = {
  state: SyntheticTextInputState;
  messages: SyntheticInputMessage[];
  domValue: TextEditingValue;
};

export type ReconcileTextInputSnapshot = {
  collapsed: boolean;
  nodeId: string | null;
  cursorOffset: number;
  precedingText: string;
  followingText: string;
};

export type ReconcileTextInputResult = {
  state: SyntheticTextInputState;
  shouldCommitPreedit: boolean;
  domValue: TextEditingValue;
};

export const createCollapsedSelection = (offset: number): TextSelection => ({
  baseOffset: offset,
  extentOffset: offset,
});

export const createTextEditingValue = (
  text = '',
  selection: TextSelection = createCollapsedSelection(0),
  composing: TextRange = EMPTY_COMPOSING,
): TextEditingValue => ({
  text,
  selection,
  composing: { ...composing },
});

export const createSyntheticTextInputState = (currentValue: TextEditingValue = createTextEditingValue()): SyntheticTextInputState => ({
  currentValue,
  sentinelLost: false,
  hadDeltaSinceReconcile: false,
  extendedComposingStart: null,
  reconcileNodeId: null,
  reconcileCursorOffset: null,
});

export const createPendingDeltaState = (): PendingDeltaState => ({
  inputType: null,
  data: null,
  deltaText: '',
  deltaStart: -1,
  deltaEnd: -1,
});

export const createCompositionTrackingState = (): CompositionTrackingState => ({
  active: false,
  text: null,
  base: null,
  replaceStart: null,
  replaceEnd: null,
});

export const resetCompositionTracking = (): CompositionTrackingState => createCompositionTrackingState();

export const startCompositionTracking = (replacementRange?: TextRange | null): CompositionTrackingState => {
  const hasReplacementRange = replacementRange != null && replacementRange.start >= 0 && replacementRange.end > replacementRange.start;

  return {
    active: true,
    text: null,
    base: hasReplacementRange ? replacementRange.start : null,
    replaceStart: hasReplacementRange ? replacementRange.start : null,
    replaceEnd: hasReplacementRange ? replacementRange.end : null,
  };
};

export const updateCompositionTracking = (
  tracking: CompositionTrackingState,
  text: string | null | undefined,
): CompositionTrackingState => ({
  active: true,
  text: text == null ? null : normalizeInsertedText(text),
  base: tracking.base,
  replaceStart: tracking.replaceStart,
  replaceEnd: tracking.replaceEnd,
});

const cloneSelection = (selection: TextSelection): TextSelection => ({
  baseOffset: selection.baseOffset,
  extentOffset: selection.extentOffset,
});

const cloneRange = (range: TextRange): TextRange => ({
  start: range.start,
  end: range.end,
});

const cloneValue = (value: TextEditingValue): TextEditingValue => ({
  text: value.text,
  selection: cloneSelection(value.selection),
  composing: cloneRange(value.composing),
});

const isSelectionCollapsed = (selection: TextSelection) => selection.baseOffset === selection.extentOffset;

const isRangeValid = (range: TextRange) => range.start >= 0 && range.end >= range.start;

const isRangeCollapsed = (range: TextRange) => range.start === range.end;

const hasComposingText = (value: TextEditingValue) => isRangeValid(value.composing) && !isRangeCollapsed(value.composing);

const valueEquals = (left: TextEditingValue, right: TextEditingValue) =>
  left.text === right.text &&
  left.selection.baseOffset === right.selection.baseOffset &&
  left.selection.extentOffset === right.selection.extentOffset &&
  left.composing.start === right.composing.start &&
  left.composing.end === right.composing.end;

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

const stripSentinel = (text: string) => (text.startsWith(SENTINEL) ? text.slice(SENTINEL.length) : text);

const normalizeInsertedText = (text: string) => text.replaceAll(/\r\n?/g, '\n');

export const countTextChars = (text: string) => (graphemeSegmenter ? [...graphemeSegmenter.segment(text)].length : [...text].length);

export const positionMarker = (nodeId: string, cursorOffset: number) =>
  String.fromCodePoint(MARKER_BASE + ((hashString(nodeId) ^ cursorOffset) & 0xf_ff));

const hashString = (value: string) => {
  let hash = 0;
  for (const char of value) {
    hash = Math.imul(hash, 31) + (char.codePointAt(0) ?? 0);
  }
  return hash;
};

const hasTrailingMarker = (text: string) => {
  if (!text) return false;
  const lastCode = text.codePointAt(text.length - 1) ?? -1;
  return lastCode >= MARKER_BASE && lastCode < MARKER_BASE + 0x10_00;
};

const stripTrailingMarker = (text: string) => (hasTrailingMarker(text) ? text.slice(0, -1) : text);

const projectOffsetToDom = (offset: number, hasSentinel: boolean, maxLength: number) => {
  if (offset < 0) {
    return offset;
  }

  const projectedOffset = hasSentinel ? offset - SENTINEL.length : offset;
  return clamp(projectedOffset, 0, maxLength);
};

const restoreOffsetFromDom = (offset: number, hasSentinel: boolean, maxLength: number) => {
  if (offset < 0) {
    return offset;
  }

  const restoredOffset = hasSentinel ? offset + SENTINEL.length : offset;
  return clamp(restoredOffset, 0, maxLength);
};

const projectTextEditingValueToDom = (value: TextEditingValue): TextEditingValue => {
  const hasSentinel = value.text.startsWith(SENTINEL);
  const domText = stripTrailingMarker(stripSentinel(value.text));

  return createTextEditingValue(
    domText,
    {
      baseOffset: projectOffsetToDom(value.selection.baseOffset, hasSentinel, domText.length),
      extentOffset: projectOffsetToDom(value.selection.extentOffset, hasSentinel, domText.length),
    },
    isRangeValid(value.composing)
      ? {
          start: projectOffsetToDom(value.composing.start, hasSentinel, domText.length),
          end: projectOffsetToDom(value.composing.end, hasSentinel, domText.length),
        }
      : EMPTY_COMPOSING,
  );
};

const restoreTextEditingValueFromDom = (domValue: TextEditingValue, referenceValue: TextEditingValue): TextEditingValue => {
  const hasSentinel = referenceValue.text.startsWith(SENTINEL);
  const marker = hasTrailingMarker(referenceValue.text) ? (referenceValue.text.at(-1) ?? '') : '';
  const restoredText = `${hasSentinel ? SENTINEL : ''}${domValue.text}${marker}`;

  return createTextEditingValue(
    restoredText,
    {
      baseOffset: restoreOffsetFromDom(domValue.selection.baseOffset, hasSentinel, restoredText.length),
      extentOffset: restoreOffsetFromDom(domValue.selection.extentOffset, hasSentinel, restoredText.length),
    },
    isRangeValid(domValue.composing)
      ? {
          start: restoreOffsetFromDom(domValue.composing.start, hasSentinel, restoredText.length),
          end: restoreOffsetFromDom(domValue.composing.end, hasSentinel, restoredText.length),
        }
      : EMPTY_COMPOSING,
  );
};

const withSelection = (value: TextEditingValue, selection: TextSelection, composing = value.composing): TextEditingValue => ({
  text: value.text,
  selection: cloneSelection(selection),
  composing: cloneRange(composing),
});

const getSelectionEnd = (selection: TextSelection) => Math.max(selection.baseOffset, selection.extentOffset);

const getSuffixPrefixOverlap = (prefix: string, text: string) => {
  const maxLength = Math.min(prefix.length, text.length);
  for (let length = maxLength; length > 0; length -= 1) {
    if (prefix.endsWith(text.slice(0, length))) {
      return length;
    }
  }
  return 0;
};

const normalizeSelectionForComposition = (selection: TextSelection, start: number, end: number): TextSelection =>
  isSelectionCollapsed(selection) ? createCollapsedSelection(end) : { baseOffset: start, extentOffset: end };

const hasReplacementRange = (tracking: CompositionTrackingState) =>
  tracking.replaceStart != null && tracking.replaceEnd != null && tracking.replaceStart >= 0 && tracking.replaceEnd > tracking.replaceStart;

const shouldPreserveTrackedText = (tracking: CompositionTrackingState, incomingText: string, fallbackText: string) =>
  hasReplacementRange(tracking) && fallbackText.length > 0 && incomingText.trim().length === 0;

const inferCompositionRangeFromDomDiff = (
  rawValue: TextEditingValue,
  tracking: CompositionTrackingState,
  referenceDomValue: TextEditingValue,
): { value: TextEditingValue; tracking: CompositionTrackingState } | null => {
  if (!tracking.active || tracking.text == null) {
    return null;
  }

  const diff = diffText(referenceDomValue.text, rawValue.text);
  if (diff.removedText.length === 0 && diff.insertedText.length === 0) {
    return null;
  }

  const cursor = getSelectionEnd(referenceDomValue.selection);
  const prefix = referenceDomValue.text.slice(0, cursor);
  const suffix = referenceDomValue.text.slice(cursor);
  const duplicatedText = prefix + tracking.text + suffix;
  const overlap = getSuffixPrefixOverlap(prefix, tracking.text);
  if (overlap > 0 && overlap < tracking.text.length && rawValue.text === duplicatedText) {
    return null;
  }

  const start = diff.prefix;
  const end = start + diff.removedText.length;
  const replacedText = referenceDomValue.text.slice(start, end);
  const effectiveText = shouldPreserveTrackedText(tracking, diff.insertedText, replacedText)
    ? replacedText
    : diff.insertedText.length > 0
      ? diff.insertedText
      : tracking.text;

  return {
    value: createTextEditingValue(
      referenceDomValue.text.slice(0, start) + effectiveText + referenceDomValue.text.slice(end),
      normalizeSelectionForComposition(rawValue.selection, start, start + effectiveText.length),
      {
        start,
        end: start + effectiveText.length,
      },
    ),
    tracking: {
      ...tracking,
      text: effectiveText,
      base: start,
    },
  };
};

const stripRevivedPrefixFromActiveComposition = (
  rawValue: TextEditingValue,
  prefix: string,
  suffix: string,
  currentCompositionText: string,
  trackingText: string,
) => {
  if (
    currentCompositionText.length === 0 ||
    trackingText.length <= currentCompositionText.length ||
    !trackingText.endsWith(currentCompositionText) ||
    rawValue.text !== prefix + trackingText + suffix
  ) {
    return trackingText;
  }

  const prependedText = trackingText.slice(0, trackingText.length - currentCompositionText.length);
  if (prependedText.length === 0 || prefix.endsWith(prependedText)) {
    return trackingText;
  }

  return currentCompositionText;
};

const inferExpandedActiveCompositionFromDuplicatedPrefix = (
  rawValue: TextEditingValue,
  tracking: CompositionTrackingState,
  referenceDomValue: TextEditingValue,
  prefix: string,
  suffix: string,
  currentCompositionText: string,
  start: number,
) => {
  if (!tracking.active || tracking.text == null || tracking.text.length === 0) {
    return null;
  }

  // (일본어 재조합 케이스처럼) IME가 넓어진 후보 범위를 실제로 선택 중일 때만 앞쪽 중복 텍스트를
  // 확장된 조합 범위로 본다. 커서가 collapsed 상태라면 같은 prefix는 이미 확정된
  // 일반 문맥이므로 현재 preedit 범위에 포함되면 안 된다.
  if (isSelectionCollapsed(rawValue.selection)) {
    return null;
  }

  if (currentCompositionText.length > 0 && tracking.text.startsWith(currentCompositionText)) {
    return null;
  }

  if (tracking.text.length <= currentCompositionText.length && rawValue.text.length <= referenceDomValue.text.length) {
    return null;
  }

  const trailingText = `${tracking.text}${suffix}`;
  if (!rawValue.text.endsWith(trailingText)) {
    return null;
  }

  const duplicatedPrefixLength = rawValue.text.length - trailingText.length;
  if (duplicatedPrefixLength <= 0) {
    return null;
  }

  const duplicatedPrefix = rawValue.text.slice(0, duplicatedPrefixLength);
  if (!prefix.endsWith(duplicatedPrefix)) {
    return null;
  }

  const expandedStart = start - duplicatedPrefix.length;
  if (expandedStart < 0 || expandedStart >= start) {
    return null;
  }

  return {
    start: expandedStart,
    text: tracking.text,
  };
};

const normalizeDomValueForComposition = (
  rawValue: TextEditingValue,
  tracking: CompositionTrackingState,
  referenceValue: TextEditingValue,
): { value: TextEditingValue; tracking: CompositionTrackingState } => {
  const referenceDomValue = projectTextEditingValueToDom(referenceValue);
  if ((!tracking.active || tracking.text == null) && !hasComposingText(referenceDomValue)) {
    return { value: rawValue, tracking };
  }

  if (hasComposingText(referenceDomValue)) {
    const start = referenceDomValue.composing.start;
    if (rawValue.text === referenceDomValue.text) {
      return { value: rawValue, tracking };
    }

    const prefix = referenceDomValue.text.slice(0, start);
    const suffix = referenceDomValue.text.slice(referenceDomValue.composing.end);
    const currentCompositionText = referenceDomValue.text.slice(start, referenceDomValue.composing.end);
    const inferredComposition = inferCompositionRangeFromDomDiff(rawValue, tracking, referenceDomValue);

    if (inferredComposition && inferredComposition.tracking.text === tracking.text && inferredComposition.tracking.base === start) {
      return inferredComposition;
    }

    const expandedComposition = inferExpandedActiveCompositionFromDuplicatedPrefix(
      rawValue,
      tracking,
      referenceDomValue,
      prefix,
      suffix,
      currentCompositionText,
      start,
    );
    if (expandedComposition) {
      const effectiveText = shouldPreserveTrackedText(tracking, expandedComposition.text, currentCompositionText)
        ? currentCompositionText
        : expandedComposition.text;

      return {
        value: createTextEditingValue(
          referenceDomValue.text.slice(0, expandedComposition.start) + effectiveText + suffix,
          normalizeSelectionForComposition(rawValue.selection, expandedComposition.start, expandedComposition.start + effectiveText.length),
          tracking.active
            ? {
                start: expandedComposition.start,
                end: expandedComposition.start + effectiveText.length,
              }
            : EMPTY_COMPOSING,
        ),
        tracking: tracking.active
          ? {
              ...tracking,
              text: effectiveText,
              base: expandedComposition.start,
            }
          : tracking,
      };
    }

    const candidateCompositionEnd = Math.max(prefix.length, rawValue.text.length - suffix.length);
    const candidateCompositionText = (tracking.text ?? rawValue.text.slice(prefix.length, candidateCompositionEnd)).slice(
      0,
      Math.max(0, candidateCompositionEnd - prefix.length),
    );
    const normalizedTrackingText = stripRevivedPrefixFromActiveComposition(
      rawValue,
      prefix,
      suffix,
      currentCompositionText,
      candidateCompositionText,
    );
    const effectiveText = shouldPreserveTrackedText(tracking, normalizedTrackingText, currentCompositionText)
      ? currentCompositionText
      : normalizedTrackingText;
    const composingRange = tracking.active
      ? {
          start,
          end: start + effectiveText.length,
        }
      : EMPTY_COMPOSING;

    return {
      value: createTextEditingValue(
        prefix + effectiveText + suffix,
        normalizeSelectionForComposition(rawValue.selection, start, start + effectiveText.length),
        composingRange,
      ),
      tracking: tracking.active
        ? {
            ...tracking,
            text: effectiveText,
            base: start,
          }
        : tracking,
    };
  }

  if (hasReplacementRange(tracking)) {
    const start = clamp(tracking.replaceStart ?? 0, 0, referenceDomValue.text.length);
    const end = clamp(tracking.replaceEnd ?? start, start, referenceDomValue.text.length);
    const prefix = referenceDomValue.text.slice(0, start);
    const suffix = referenceDomValue.text.slice(end);
    const replacedText = referenceDomValue.text.slice(start, end);
    const incomingText = tracking.text ?? rawValue.text;
    const effectiveText = shouldPreserveTrackedText(tracking, incomingText, replacedText) ? replacedText : incomingText;

    return {
      value: createTextEditingValue(
        prefix + effectiveText + suffix,
        normalizeSelectionForComposition(rawValue.selection, start, start + effectiveText.length),
        {
          start,
          end: start + effectiveText.length,
        },
      ),
      tracking: {
        ...tracking,
        text: effectiveText,
        base: start,
      },
    };
  }

  const inferredComposition = inferCompositionRangeFromDomDiff(rawValue, tracking, referenceDomValue);
  if (inferredComposition) {
    return inferredComposition;
  }

  if (tracking.text == null) {
    return { value: rawValue, tracking };
  }

  const cursor = getSelectionEnd(referenceDomValue.selection);
  const prefix = referenceDomValue.text.slice(0, cursor);
  const suffix = referenceDomValue.text.slice(cursor);
  const overlap = getSuffixPrefixOverlap(prefix, tracking.text);
  const duplicatedCompositionText = prefix + tracking.text + suffix;

  if (overlap === 0 || overlap >= tracking.text.length || rawValue.text !== duplicatedCompositionText) {
    return { value: rawValue, tracking };
  }

  const effectiveText = tracking.text.slice(overlap);
  const start = cursor;
  return {
    value: createTextEditingValue(
      prefix + effectiveText + suffix,
      normalizeSelectionForComposition(rawValue.selection, start, start + effectiveText.length),
      {
        start,
        end: start + effectiveText.length,
      },
    ),
    tracking: {
      ...tracking,
      text: effectiveText,
      base: start,
    },
  };
};

const setCurrentValue = (state: SyntheticTextInputState, value: TextEditingValue, allowSentinel = true): SyntheticTextInputState => {
  if (valueEquals(state.currentValue, value)) {
    return state;
  }

  const nextValue = cloneValue(value);
  const shouldAddSentinel =
    allowSentinel &&
    !nextValue.text.startsWith(SENTINEL) &&
    ((state.sentinelLost && !hasComposingText(nextValue)) || nextValue.selection.baseOffset === 0);

  if (!shouldAddSentinel) {
    return {
      ...state,
      currentValue: nextValue,
    };
  }

  return {
    ...state,
    currentValue: {
      text: SENTINEL + nextValue.text,
      selection:
        nextValue.selection.baseOffset >= 0
          ? {
              baseOffset: nextValue.selection.baseOffset + 1,
              extentOffset: nextValue.selection.extentOffset + 1,
            }
          : cloneSelection(nextValue.selection),
      composing: isRangeValid(nextValue.composing)
        ? {
            start: nextValue.composing.start + 1,
            end: nextValue.composing.end + 1,
          }
        : cloneRange(nextValue.composing),
    },
    sentinelLost: false,
  };
};

const effectiveContentDiffers = (currentValue: TextEditingValue, precedingText: string, followingText: string) => {
  let text = currentValue.text;
  let cursor = currentValue.selection.baseOffset;

  if (cursor < 0) {
    return true;
  }

  if (text.startsWith(SENTINEL)) {
    text = text.slice(SENTINEL.length);
    cursor -= SENTINEL.length;
  }

  text = stripTrailingMarker(text);
  cursor = clamp(cursor, 0, text.length);

  const keyboardBefore = text.slice(0, cursor);
  const keyboardAfter = text.slice(cursor);

  const beforeOk =
    (keyboardBefore.length === 0 && precedingText.length === 0) ||
    (keyboardBefore.length > 0 &&
      precedingText.length > 0 &&
      (keyboardBefore.endsWith(precedingText) || precedingText.endsWith(keyboardBefore)));
  const afterOk =
    (keyboardAfter.length === 0 && followingText.length === 0) ||
    (keyboardAfter.length > 0 &&
      followingText.length > 0 &&
      (keyboardAfter.startsWith(followingText) || followingText.startsWith(keyboardAfter)));

  return !(beforeOk && afterOk);
};

const isStaleWindow = (state: SyntheticTextInputState, text: string) => {
  if (!text || state.reconcileNodeId == null || state.reconcileCursorOffset == null) {
    return false;
  }

  if (!hasTrailingMarker(text)) {
    return false;
  }

  return (
    (text.codePointAt(text.length - 1) ?? -1) !== (positionMarker(state.reconcileNodeId, state.reconcileCursorOffset).codePointAt(0) ?? -1)
  );
};

export const applyTextEditingDelta = (delta: SyntheticTextEditingDelta, value: TextEditingValue): TextEditingValue => {
  switch (delta.type) {
    case 'insertion': {
      return {
        text: value.text.slice(0, delta.insertionOffset) + delta.textInserted + value.text.slice(delta.insertionOffset),
        selection: cloneSelection(delta.selection),
        composing: cloneRange(delta.composing),
      };
    }
    case 'deletion': {
      return {
        text: value.text.slice(0, delta.deletedRange.start) + value.text.slice(delta.deletedRange.end),
        selection: cloneSelection(delta.selection),
        composing: cloneRange(delta.composing),
      };
    }
    case 'replacement': {
      return {
        text: value.text.slice(0, delta.replacedRange.start) + delta.replacementText + value.text.slice(delta.replacedRange.end),
        selection: cloneSelection(delta.selection),
        composing: cloneRange(delta.composing),
      };
    }
    case 'nonTextUpdate': {
      return {
        text: value.text,
        selection: cloneSelection(delta.selection),
        composing: cloneRange(delta.composing),
      };
    }
  }
};

const createInsertionDelta = (
  oldText: string,
  textInserted: string,
  insertionOffset: number,
  selection: TextSelection,
  composing: TextRange,
): TextEditingDeltaInsertion => ({
  type: 'insertion',
  oldText,
  textInserted,
  insertionOffset,
  selection: cloneSelection(selection),
  composing: cloneRange(composing),
});

const createDeletionDelta = (
  oldText: string,
  deletedRange: TextRange,
  selection: TextSelection,
  composing: TextRange,
): TextEditingDeltaDeletion => ({
  type: 'deletion',
  oldText,
  deletedRange: cloneRange(deletedRange),
  selection: cloneSelection(selection),
  composing: cloneRange(composing),
});

const createReplacementDelta = (
  oldText: string,
  replacedRange: TextRange,
  replacementText: string,
  selection: TextSelection,
  composing: TextRange,
): TextEditingDeltaReplacement => ({
  type: 'replacement',
  oldText,
  replacedRange: cloneRange(replacedRange),
  replacementText,
  selection: cloneSelection(selection),
  composing: cloneRange(composing),
});

const createNonTextUpdateDelta = (oldText: string, selection: TextSelection, composing: TextRange): TextEditingDeltaNonTextUpdate => ({
  type: 'nonTextUpdate',
  oldText,
  selection: cloneSelection(selection),
  composing: cloneRange(composing),
});

const replaceText = (text: string, replacementText: string, range: TextRange) =>
  text.slice(0, range.start) + replacementText + text.slice(range.end);

const diffText = (before: string, after: string) => {
  const minLen = Math.min(before.length, after.length);
  let prefix = 0;
  while (prefix < minLen && before[prefix] === after[prefix]) {
    prefix += 1;
  }

  let beforeEnd = before.length;
  let afterEnd = after.length;
  while (beforeEnd > prefix && afterEnd > prefix && before[beforeEnd - 1] === after[afterEnd - 1]) {
    beforeEnd -= 1;
    afterEnd -= 1;
  }

  return {
    prefix,
    removedText: before.slice(prefix, beforeEnd),
    insertedText: after.slice(prefix, afterEnd),
  };
};

export const createBeforeInputState = (
  currentValue: TextEditingValue,
  inputType: string | null,
  data: string | null,
): PendingDeltaState => {
  const normalizedData = data == null ? null : normalizeInsertedText(data);

  if (inputType == null) {
    return createPendingDeltaState();
  }

  const isSelectionInverted = currentValue.selection.baseOffset > currentValue.selection.extentOffset;
  const deltaOffset = isSelectionInverted ? currentValue.selection.baseOffset : currentValue.selection.extentOffset;

  if (inputType.includes('delete')) {
    return {
      inputType,
      data: normalizedData,
      deltaText: '',
      deltaStart: -1,
      deltaEnd: deltaOffset,
    };
  }

  if (inputType === 'insertLineBreak') {
    return {
      inputType,
      data: '\n',
      deltaText: '\n',
      deltaStart: deltaOffset,
      deltaEnd: deltaOffset,
    };
  }

  if (normalizedData != null) {
    return {
      inputType,
      data: normalizedData,
      deltaText: normalizedData,
      deltaStart: deltaOffset,
      deltaEnd: deltaOffset,
    };
  }

  return {
    inputType,
    data: normalizedData,
    deltaText: '',
    deltaStart: -1,
    deltaEnd: -1,
  };
};

export const withCompositionTracking = (
  value: TextEditingValue,
  tracking: CompositionTrackingState,
): { value: TextEditingValue; tracking: CompositionTrackingState } => {
  if (!tracking.active || tracking.text == null) {
    return { value, tracking };
  }

  const nextBase = tracking.base != null && tracking.base >= 0 ? tracking.base : value.selection.extentOffset - tracking.text.length;

  if (nextBase < 0) {
    return {
      value,
      tracking: {
        ...tracking,
        base: null,
      },
    };
  }

  const nextSelection = createCollapsedSelection(nextBase + tracking.text.length);

  return {
    value: createTextEditingValue(value.text, isSelectionCollapsed(value.selection) ? nextSelection : value.selection, {
      start: nextBase,
      end: nextBase + tracking.text.length,
    }),
    tracking: {
      ...tracking,
      base: nextBase,
    },
  };
};

export const readDomTextEditingValue = (
  element: HTMLInputElement | HTMLTextAreaElement,
  tracking: CompositionTrackingState,
  referenceValue: TextEditingValue,
): { value: TextEditingValue; tracking: CompositionTrackingState; shouldSyncDom: boolean } => {
  const baseOffset = element.selectionStart ?? 0;
  const extentOffset = element.selectionEnd ?? baseOffset;
  const rawValue = createTextEditingValue(normalizeInsertedText(element.value), { baseOffset, extentOffset }, EMPTY_COMPOSING);
  const normalizedValue = normalizeDomValueForComposition(rawValue, tracking, referenceValue);
  const trackedValue = withCompositionTracking(normalizedValue.value, normalizedValue.tracking);

  return {
    value: restoreTextEditingValueFromDom(trackedValue.value, referenceValue),
    tracking: trackedValue.tracking,
    shouldSyncDom: rawValue.text !== trackedValue.value.text,
  };
};

const createDeltaFromRange = (
  oldValue: TextEditingValue,
  newValue: TextEditingValue,
  deltaStart: number,
  deltaEnd: number,
  deltaText: string,
): SyntheticTextEditingDelta => {
  if (deltaText.length === 0 && deltaStart === deltaEnd) {
    return createNonTextUpdateDelta(oldValue.text, newValue.selection, newValue.composing);
  }

  if (deltaText.length === 0) {
    return createDeletionDelta(oldValue.text, { start: deltaStart, end: deltaEnd }, newValue.selection, newValue.composing);
  }

  if (deltaStart === deltaEnd) {
    return createInsertionDelta(oldValue.text, deltaText, deltaStart, newValue.selection, newValue.composing);
  }

  return createReplacementDelta(oldValue.text, { start: deltaStart, end: deltaEnd }, deltaText, newValue.selection, newValue.composing);
};

export const inferSyntheticTextEditingDeltas = (
  oldValue: TextEditingValue,
  newValue: TextEditingValue,
  pendingDelta: PendingDeltaState,
): SyntheticTextEditingDelta[] => {
  if (valueEquals(oldValue, newValue)) {
    return [];
  }

  if (oldValue.text === newValue.text) {
    return [createNonTextUpdateDelta(oldValue.text, newValue.selection, newValue.composing)];
  }

  let deltaText = pendingDelta.deltaText;
  let deltaStart = pendingDelta.deltaStart;
  let deltaEnd = pendingDelta.deltaEnd;
  const previousSelectionWasCollapsed = isSelectionCollapsed(oldValue.selection);
  const isTextBeingRemoved = deltaText.length === 0 && deltaEnd !== -1;
  const isTextBeingChangedAtActiveSelection = deltaText.length > 0 && !previousSelectionWasCollapsed;

  if (isTextBeingRemoved) {
    const deletedLength = oldValue.text.length - newValue.text.length;
    const backwardDeletion = newValue.selection.baseOffset !== oldValue.selection.baseOffset;
    if (backwardDeletion) {
      deltaStart = deltaEnd - deletedLength;
    } else {
      deltaStart = newValue.selection.baseOffset;
      deltaEnd = deltaStart + deletedLength;
    }
  } else if (isTextBeingChangedAtActiveSelection) {
    deltaStart = Math.min(oldValue.selection.baseOffset, oldValue.selection.extentOffset);
    deltaEnd = Math.max(oldValue.selection.baseOffset, oldValue.selection.extentOffset);
  }

  const isCurrentlyComposing = hasComposingText(newValue);
  if (deltaText.length > 0 && previousSelectionWasCollapsed && isCurrentlyComposing) {
    deltaStart = newValue.composing.start;
    if (deltaEnd === -1) {
      deltaEnd = deltaStart;
    }
  }

  const hasCandidateRange = deltaStart >= 0 && deltaEnd >= deltaStart && deltaEnd <= oldValue.text.length;
  const candidateMatches =
    hasCandidateRange &&
    replaceText(oldValue.text, deltaText, {
      start: deltaStart,
      end: deltaEnd,
    }) === newValue.text;

  if (!candidateMatches) {
    const diff = diffText(oldValue.text, newValue.text);
    deltaStart = diff.prefix;
    deltaEnd = diff.prefix + diff.removedText.length;
    deltaText = diff.insertedText;
  }

  return [createDeltaFromRange(oldValue, newValue, deltaStart, deltaEnd, deltaText)];
};

const inputMessage = (text: string): SyntheticInputMessage => ({ type: 'input', text });

const deleteBackwardMessage = (length = 1): SyntheticInputMessage =>
  length > 1 ? { type: 'deleteBackward', length } : { type: 'deleteBackward' };

const replaceBackwardMessage = (length: number, text: string): SyntheticInputMessage => ({
  type: 'replaceBackward',
  length,
  text,
});

const compositionStartMessage = (text: string): SyntheticInputMessage => ({ type: 'compositionStart', text });

const compositionUpdateMessage = (text: string, replaceLength = 0): SyntheticInputMessage =>
  replaceLength > 0 ? { type: 'compositionUpdate', text, replaceLength } : { type: 'compositionUpdate', text };

const commitPreeditMessage = (): SyntheticInputMessage => ({ type: 'commitPreedit' });

const compositionEndMessage = (): SyntheticInputMessage => ({ type: 'compositionEnd' });

const insertNewlineMessage = (): SyntheticInputMessage => ({ type: 'insertNewline' });

const navigateMessage = (direction: string): SyntheticInputMessage => ({
  type: 'navigate',
  direction,
  extend: false,
});

const clearComposing = (value: TextEditingValue): TextEditingValue => ({
  ...value,
  composing: cloneRange(EMPTY_COMPOSING),
});

const countReplacedTextChars = (value: TextEditingValue, start: number, end: number) => {
  const effectiveText = stripTrailingMarker(stripSentinel(value.text));
  const sentinelLength = value.text.startsWith(SENTINEL) ? SENTINEL.length : 0;
  const effectiveStart = clamp(start - sentinelLength, 0, effectiveText.length);
  const effectiveEnd = clamp(end - sentinelLength, effectiveStart, effectiveText.length);
  return countTextChars(effectiveText.slice(effectiveStart, effectiveEnd));
};

const countExpandedCompositionReplaceLength = (oldValue: TextEditingValue, start: number) => {
  if (!hasComposingText(oldValue) || start >= oldValue.composing.start) {
    return 0;
  }

  const selectionStart = Math.min(oldValue.selection.baseOffset, oldValue.selection.extentOffset);
  const selectionEnd = Math.max(oldValue.selection.baseOffset, oldValue.selection.extentOffset);
  const selectionRemovesCurrentComposition =
    !isSelectionCollapsed(oldValue.selection) && selectionStart <= oldValue.composing.start && selectionEnd >= oldValue.composing.end;

  return countReplacedTextChars(oldValue, start, selectionRemovesCurrentComposition ? oldValue.composing.start : oldValue.composing.end);
};

export const reduceEditingDeltas = (state: SyntheticTextInputState, deltas: SyntheticTextEditingDelta[]): ReduceEditingDeltasResult => {
  const oldValue = state.currentValue;
  const newValue = deltas.reduce((value, delta) => applyTextEditingDelta(delta, value), oldValue);
  let nextState = { ...state };
  const messages: SyntheticInputMessage[] = [];

  if (oldValue.text.startsWith(SENTINEL) && !newValue.text.startsWith(SENTINEL)) {
    nextState.sentinelLost = true;
  }

  if (isStaleWindow(state, oldValue.text) || (deltas.length > 0 && isStaleWindow(state, deltas[0].oldText))) {
    const currentValue = hasComposingText(nextState.currentValue) ? clearComposing(nextState.currentValue) : nextState.currentValue;
    nextState = {
      ...nextState,
      currentValue,
      extendedComposingStart: null,
    };
    return {
      state: nextState,
      messages,
      domValue: nextState.currentValue,
    };
  }

  const hadComposing = hasComposingText(oldValue);
  const hasComposing = hasComposingText(newValue);

  let midComposing: TextRange | null = null;
  if (!hadComposing && !hasComposing) {
    let scan = oldValue;
    for (const delta of deltas) {
      const previousText = scan.text;
      scan = applyTextEditingDelta(delta, scan);
      if (hasComposingText(scan)) {
        if (previousText !== scan.text) {
          midComposing = cloneRange(scan.composing);
        }
        break;
      }
    }
  }

  if (hadComposing || hasComposing) {
    if (hasComposing) {
      const start = newValue.composing.start;
      const text = newValue.text.slice(start, newValue.composing.end);
      if (hadComposing) {
        const replaceLength = countExpandedCompositionReplaceLength(oldValue, start);
        messages.push(replaceLength > 0 ? compositionUpdateMessage(text, replaceLength) : compositionUpdateMessage(text));
      } else {
        const replaceLength = countReplacedTextChars(oldValue, start, Math.max(getSelectionEnd(oldValue.selection), start));
        messages.push(replaceLength > 0 ? compositionUpdateMessage(text, replaceLength) : compositionStartMessage(text));
      }
    } else {
      if (oldValue.text === newValue.text) {
        messages.push(commitPreeditMessage());
      } else {
        const composingStart = oldValue.composing.start;
        const prefix = oldValue.text.slice(0, composingStart);
        const suffix = oldValue.text.slice(oldValue.composing.end);
        const committed = newValue.text.slice(prefix.length, newValue.text.length - suffix.length);
        const hasNewline = committed.includes('\n');
        const committedText = hasNewline ? committed.replaceAll('\n', '') : committed;

        if (committedText.length === 0) {
          messages.push(compositionEndMessage());
        } else {
          messages.push(compositionUpdateMessage(committedText), commitPreeditMessage());
        }

        if (hasNewline) {
          messages.push(insertNewlineMessage());
          nextState = setCurrentValue(
            {
              ...nextState,
              extendedComposingStart: null,
            },
            createTextEditingValue(
              prefix + committedText + suffix,
              createCollapsedSelection(prefix.length + committedText.length),
              EMPTY_COMPOSING,
            ),
          );
          return {
            state: nextState,
            messages,
            domValue: nextState.currentValue,
          };
        }
      }

      nextState.extendedComposingStart = null;
    }
  } else if (midComposing && oldValue.text !== newValue.text) {
    let committed = newValue.text.slice(midComposing.start, midComposing.end);
    const replaceStart = clamp(midComposing.start, 0, oldValue.text.length);
    const replaceEnd = clamp(midComposing.end, replaceStart, oldValue.text.length);
    let replaceTextValue = oldValue.text.slice(replaceStart, replaceEnd);

    committed = stripTrailingMarker(committed);
    replaceTextValue = stripTrailingMarker(replaceTextValue);

    messages.push(compositionUpdateMessage(committed, countTextChars(replaceTextValue)), commitPreeditMessage());
  } else if (newValue.text.length === 0) {
    messages.push(deleteBackwardMessage(oldValue.selection.baseOffset));
  } else if (!deltas.every((delta) => delta.type === 'nonTextUpdate')) {
    const effectiveOld = stripSentinel(oldValue.text);
    const effectiveNew = stripSentinel(newValue.text);

    if (effectiveOld !== effectiveNew) {
      const minLen = Math.min(effectiveOld.length, effectiveNew.length);
      let commonPrefix = 0;
      while (commonPrefix < minLen && effectiveOld[commonPrefix] === effectiveNew[commonPrefix]) {
        commonPrefix += 1;
      }

      const oldSentinelLen = oldValue.text.length - effectiveOld.length;
      const newSentinelLen = newValue.text.length - effectiveNew.length;
      const oldCursor = clamp(oldValue.selection.baseOffset - oldSentinelLen, commonPrefix, effectiveOld.length);
      const newCursor = clamp(newValue.selection.baseOffset - newSentinelLen, commonPrefix, effectiveNew.length);

      let insertedText = effectiveNew.slice(commonPrefix, newCursor);
      const removedLen = countTextChars(effectiveOld.slice(commonPrefix, oldCursor));

      if (insertedText.length === 0 && removedLen === 0) {
        const hasComplexTextDelta = deltas.some((delta) => delta.type === 'deletion' || delta.type === 'replacement');
        const insertionDeltas = deltas.filter((delta): delta is TextEditingDeltaInsertion => delta.type === 'insertion');
        if (!hasComplexTextDelta && insertionDeltas.length === 1) {
          insertedText = insertionDeltas[0].textInserted;
        }
      }

      if (insertedText.includes('\n')) {
        messages.push(insertNewlineMessage());
        nextState = setCurrentValue(nextState, oldValue);
        return {
          state: nextState,
          messages,
          domValue: nextState.currentValue,
        };
      }

      if (removedLen > 0 && insertedText.length > 0) {
        messages.push(replaceBackwardMessage(removedLen, insertedText));
      } else if (removedLen > 0) {
        messages.push(deleteBackwardMessage(removedLen));
      } else if (insertedText.length > 0) {
        messages.push(inputMessage(insertedText));
      }
    } else if (oldValue.text.startsWith(SENTINEL) && !newValue.text.startsWith(SENTINEL)) {
      messages.push(deleteBackwardMessage());
    }
  } else if (isSelectionCollapsed(oldValue.selection) && isSelectionCollapsed(newValue.selection)) {
    const oldMinOffset = oldValue.text.startsWith(SENTINEL) ? SENTINEL.length : 0;
    const newMinOffset = newValue.text.startsWith(SENTINEL) ? SENTINEL.length : 0;
    const effectiveOldOffset = Math.max(oldValue.selection.baseOffset, oldMinOffset);
    const effectiveNewOffset = Math.max(newValue.selection.baseOffset, newMinOffset);
    const delta = effectiveNewOffset - effectiveOldOffset;
    if (delta !== 0) {
      for (let i = 0; i < Math.abs(delta); i += 1) {
        messages.push(navigateMessage(delta > 0 ? 'right' : 'left'));
      }
    }

    if (newValue.selection.baseOffset < newMinOffset) {
      const crossedSentinelLeft = oldValue.selection.baseOffset >= oldMinOffset;
      if (crossedSentinelLeft) {
        messages.push(navigateMessage('left'));
      }

      nextState = setCurrentValue(nextState, withSelection(newValue, createCollapsedSelection(newMinOffset)), false);
      return {
        state: nextState,
        messages,
        domValue: nextState.currentValue,
      };
    }
  }

  nextState = setCurrentValue(nextState, newValue, false);
  nextState.hadDeltaSinceReconcile = true;

  return {
    state: nextState,
    messages,
    domValue: nextState.currentValue,
  };
};

export const invalidateSyntheticTextInputState = (state: SyntheticTextInputState): SyntheticTextInputState => {
  if (!hasComposingText(state.currentValue)) {
    return state;
  }

  return {
    ...state,
    currentValue: clearComposing(state.currentValue),
    extendedComposingStart: null,
  };
};

export const reconcileSyntheticTextInputState = (state: SyntheticTextInputState, selection: Selection | null): ReconcileTextInputResult => {
  if (!selection) {
    return {
      state,
      shouldCommitPreedit: false,
      domValue: state.currentValue,
    };
  }

  const snapshot: ReconcileTextInputSnapshot = {
    collapsed: selection.collapsed,
    nodeId: selection.anchor.nodeId,
    cursorOffset: selection.anchor.offset,
    precedingText: selection.collapsed ? selection.precedingText : '',
    followingText: selection.collapsed ? selection.followingText : '',
  };

  if (snapshot.nodeId == null) {
    return {
      state,
      shouldCommitPreedit: false,
      domValue: state.currentValue,
    };
  }

  let nextState = snapshot.collapsed ? state : invalidateSyntheticTextInputState(state);
  const marker = positionMarker(snapshot.nodeId, snapshot.cursorOffset);
  const nextValue = createTextEditingValue(
    snapshot.precedingText + snapshot.followingText + marker,
    createCollapsedSelection(snapshot.precedingText.length),
    EMPTY_COMPOSING,
  );
  let shouldCommitPreedit = false;

  if (!valueEquals(nextState.currentValue, nextValue)) {
    let forceSync = false;

    if (hasComposingText(nextState.currentValue)) {
      const stale = !nextState.hadDeltaSinceReconcile;
      nextState = {
        ...nextState,
        hadDeltaSinceReconcile: false,
      };

      if (!stale) {
        return {
          state: nextState,
          shouldCommitPreedit: false,
          domValue: nextState.currentValue,
        };
      }

      shouldCommitPreedit = true;
      nextState = {
        ...nextState,
        currentValue: clearComposing(nextState.currentValue),
        extendedComposingStart: null,
      };
      forceSync = true;
    }

    nextState = {
      ...nextState,
      hadDeltaSinceReconcile: false,
    };

    const contentChanged = effectiveContentDiffers(nextState.currentValue, snapshot.precedingText, snapshot.followingText);
    const needsSentinel = snapshot.precedingText.length === 0 && !nextState.currentValue.text.startsWith(SENTINEL);
    if (contentChanged || forceSync || needsSentinel) {
      nextState = {
        ...nextState,
        reconcileNodeId: snapshot.nodeId,
        reconcileCursorOffset: snapshot.cursorOffset,
      };
      nextState = setCurrentValue(nextState, nextValue);
    }
  } else if (snapshot.precedingText.length === 0 && !nextState.currentValue.text.startsWith(SENTINEL)) {
    nextState = {
      ...nextState,
      hadDeltaSinceReconcile: false,
      reconcileNodeId: snapshot.nodeId,
      reconcileCursorOffset: snapshot.cursorOffset,
      currentValue: createTextEditingValue(
        SENTINEL + nextValue.text,
        createCollapsedSelection(SENTINEL.length + nextValue.selection.baseOffset),
        EMPTY_COMPOSING,
      ),
      sentinelLost: false,
    };
  }

  return {
    state: nextState,
    shouldCommitPreedit,
    domValue: nextState.currentValue,
  };
};

export const syncTextEditingValueToElement = (element: HTMLInputElement | HTMLTextAreaElement, value: TextEditingValue) => {
  const domValue = projectTextEditingValueToDom(value);
  const normalizedText = normalizeInsertedText(domValue.text);
  if (element.value !== normalizedText) {
    element.value = normalizedText;
  }

  const start = clamp(domValue.selection.baseOffset, 0, normalizedText.length);
  const end = clamp(domValue.selection.extentOffset, 0, normalizedText.length);
  if (element.selectionStart !== start || element.selectionEnd !== end) {
    element.setSelectionRange(start, end);
  }
};
