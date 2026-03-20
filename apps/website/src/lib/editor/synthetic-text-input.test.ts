import { describe, expect, it } from 'vitest';
import {
  createBeforeInputState,
  createSyntheticTextInputState,
  createTextEditingValue,
  inferSyntheticTextEditingDeltas,
  positionMarker,
  readDomTextEditingValue,
  reconcileSyntheticTextInputState,
  reduceEditingDeltas,
  syncTextEditingValueToElement,
} from './synthetic-text-input';

const createCompositionTracking = (
  tracking: Partial<{
    active: boolean;
    text: string | null;
    base: number | null;
    replaceStart: number | null;
    replaceEnd: number | null;
  }>,
) => ({
  active: tracking.active ?? false,
  text: tracking.text ?? null,
  base: tracking.base ?? null,
  replaceStart: tracking.replaceStart ?? null,
  replaceEnd: tracking.replaceEnd ?? null,
});

describe('inferSyntheticTextEditingDeltas', () => {
  it('infers plain insert from beforeinput state', () => {
    const oldValue = createTextEditingValue('abc', { baseOffset: 3, extentOffset: 3 });
    const newValue = createTextEditingValue('abcd', { baseOffset: 4, extentOffset: 4 });
    const beforeInput = createBeforeInputState(oldValue, 'insertText', 'd');

    expect(inferSyntheticTextEditingDeltas(oldValue, newValue, beforeInput)).toEqual([
      {
        type: 'insertion',
        oldText: 'abc',
        textInserted: 'd',
        insertionOffset: 3,
        selection: { baseOffset: 4, extentOffset: 4 },
        composing: { start: -1, end: -1 },
      },
    ]);
  });

  it('infers backward delete from beforeinput state', () => {
    const oldValue = createTextEditingValue('abcd', { baseOffset: 4, extentOffset: 4 });
    const newValue = createTextEditingValue('abc', { baseOffset: 3, extentOffset: 3 });
    const beforeInput = createBeforeInputState(oldValue, 'deleteContentBackward', null);

    expect(inferSyntheticTextEditingDeltas(oldValue, newValue, beforeInput)).toEqual([
      {
        type: 'deletion',
        oldText: 'abcd',
        deletedRange: { start: 3, end: 4 },
        selection: { baseOffset: 3, extentOffset: 3 },
        composing: { start: -1, end: -1 },
      },
    ]);
  });

  it('infers selection replacement from beforeinput state', () => {
    const oldValue = createTextEditingValue('hello', { baseOffset: 1, extentOffset: 4 });
    const newValue = createTextEditingValue('hyo', { baseOffset: 2, extentOffset: 2 });
    const beforeInput = createBeforeInputState(oldValue, 'insertText', 'y');

    expect(inferSyntheticTextEditingDeltas(oldValue, newValue, beforeInput)).toEqual([
      {
        type: 'replacement',
        oldText: 'hello',
        replacedRange: { start: 1, end: 4 },
        replacementText: 'y',
        selection: { baseOffset: 2, extentOffset: 2 },
        composing: { start: -1, end: -1 },
      },
    ]);
  });

  it('infers line break insertion from beforeinput state', () => {
    const oldValue = createTextEditingValue('hello', { baseOffset: 2, extentOffset: 4 });
    const newValue = createTextEditingValue('he\no', { baseOffset: 3, extentOffset: 3 });
    const beforeInput = createBeforeInputState(oldValue, 'insertLineBreak', null);

    expect(inferSyntheticTextEditingDeltas(oldValue, newValue, beforeInput)).toEqual([
      {
        type: 'replacement',
        oldText: 'hello',
        replacedRange: { start: 2, end: 4 },
        replacementText: '\n',
        selection: { baseOffset: 3, extentOffset: 3 },
        composing: { start: -1, end: -1 },
      },
    ]);
  });
});

describe('reduceEditingDeltas', () => {
  it('turns non-text selection updates into navigate dispatches', () => {
    const state = createSyntheticTextInputState(createTextEditingValue('abc', { baseOffset: 1, extentOffset: 1 }));
    const result = reduceEditingDeltas(state, [
      {
        type: 'nonTextUpdate',
        oldText: 'abc',
        selection: { baseOffset: 3, extentOffset: 3 },
        composing: { start: -1, end: -1 },
      },
    ]);

    expect(result.messages).toEqual([
      { type: 'navigate', direction: 'right', extend: false },
      { type: 'navigate', direction: 'right', extend: false },
    ]);
    expect(result.state.currentValue).toEqual(createTextEditingValue('abc', { baseOffset: 3, extentOffset: 3 }));
  });

  it('re-attaches on stale marker mismatch without dispatching editor messages', () => {
    const marker = positionMarker('node-1', 3);
    const staleMarker = positionMarker('node-1', 4);
    const state = {
      ...createSyntheticTextInputState(createTextEditingValue(`abc${staleMarker}`, { baseOffset: 3, extentOffset: 3 })),
      reconcileNodeId: 'node-1',
      reconcileCursorOffset: 3,
    };
    const result = reduceEditingDeltas(state, [
      {
        type: 'insertion',
        oldText: `abc${staleMarker}`,
        textInserted: 'x',
        insertionOffset: 3,
        selection: { baseOffset: 4, extentOffset: 4 },
        composing: { start: -1, end: -1 },
      },
    ]);

    expect(result.messages).toEqual([]);
    expect(result.state.currentValue.text).toBe(`abc${staleMarker}`);
    expect(marker).not.toBe(staleMarker);
  });
});

describe('reconcileSyntheticTextInputState', () => {
  it('does not request commitPreedit for ordinary selection sync', () => {
    let state = createSyntheticTextInputState(createTextEditingValue('abcdef', { baseOffset: 3, extentOffset: 3 }));

    let result = reconcileSyntheticTextInputState(state, {
      collapsed: true,
      cmp: 0,
      selectedBlockCount: 0,
      anchor: { nodeId: 'node-1', offset: 3, affinity: 'upstream' },
      head: { nodeId: 'node-1', offset: 3, affinity: 'upstream' },
      anchorBounds: null,
      headBounds: null,
      precedingText: 'abc',
      followingText: 'def',
    });
    expect(result.shouldCommitPreedit).toBe(false);

    state = result.state;
    result = reconcileSyntheticTextInputState(state, {
      collapsed: true,
      cmp: 0,
      selectedBlockCount: 0,
      anchor: { nodeId: 'node-1', offset: 4, affinity: 'upstream' },
      head: { nodeId: 'node-1', offset: 4, affinity: 'upstream' },
      anchorBounds: null,
      headBounds: null,
      precedingText: 'abcd',
      followingText: 'ef',
    });
    expect(result.shouldCommitPreedit).toBe(false);
  });

  it('adds a sentinel at document start', () => {
    const result = reconcileSyntheticTextInputState(createSyntheticTextInputState(), {
      collapsed: true,
      cmp: 0,
      selectedBlockCount: 0,
      anchor: { nodeId: 'node-1', offset: 0, affinity: 'upstream' },
      head: { nodeId: 'node-1', offset: 0, affinity: 'upstream' },
      anchorBounds: null,
      headBounds: null,
      precedingText: '',
      followingText: 'abc',
    });

    expect(result.state.currentValue.text.startsWith('◆')).toBe(true);
    expect(result.state.currentValue.selection).toEqual({ baseOffset: 1, extentOffset: 1 });
  });
});

describe('syncTextEditingValueToElement', () => {
  it('restores the hidden input DOM state after a handled shortcut reset', () => {
    const element = document.createElement('textarea');
    element.value = '';
    syncTextEditingValueToElement(element, createTextEditingValue('abc', { baseOffset: 2, extentOffset: 2 }));

    expect(element.value).toBe('abc');
    expect(element.selectionStart).toBe(2);
    expect(element.selectionEnd).toBe(2);
  });

  it('does not write sentinel or marker characters into the DOM element', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 0);

    syncTextEditingValueToElement(element, createTextEditingValue(`◆abc${marker}`, { baseOffset: 1, extentOffset: 1 }));

    expect(element.value).toBe('abc');
    expect(element.selectionStart).toBe(0);
    expect(element.selectionEnd).toBe(0);
  });
});

describe('readDomTextEditingValue', () => {
  it('restores sentinel and marker from the reference value', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 0);
    element.value = '한abc';
    element.setSelectionRange(1, 1);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({}),
      createTextEditingValue(`◆abc${marker}`, { baseOffset: 1, extentOffset: 1 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆한abc${marker}`, { baseOffset: 2, extentOffset: 2 }));
  });

  it('tracks the composing range without mutating DOM text before input lands', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 1);
    element.value = 'ㅎ';
    element.setSelectionRange(0, 1);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '하' }),
      createTextEditingValue(`◆ㅎ${marker}`, { baseOffset: 2, extentOffset: 2 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆ㅎ${marker}`, { baseOffset: 1, extentOffset: 2 }, { start: 1, end: 2 }));
    expect(result.shouldSyncDom).toBe(false);
  });

  it('strips revived committed text when japanese reconversion duplicates the prefix', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '漢字漢字はな';
    element.setSelectionRange(2, 4);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '漢字はな' }),
      createTextEditingValue(`漢字${marker}`, { baseOffset: 2, extentOffset: 2 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`漢字はな${marker}`, { baseOffset: 2, extentOffset: 4 }, { start: 2, end: 4 }));
  });

  it('keeps repeated same-character composition after an identical committed character', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 1);
    element.value = 'ㅋㅋ';
    element.setSelectionRange(2, 2);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: 'ㅋ' }),
      createTextEditingValue(`ㅋ${marker}`, { baseOffset: 1, extentOffset: 1 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`ㅋㅋ${marker}`, { baseOffset: 2, extentOffset: 2 }, { start: 1, end: 2 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'ㅋ', base: 1 }));
    expect(result.shouldSyncDom).toBe(false);
  });

  it('keeps active composition when a later character changes to match the prefix', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 1);
    element.value = '가가';
    element.setSelectionRange(2, 2);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '가', base: 1 }),
      createTextEditingValue(`◆가각${marker}`, { baseOffset: 3, extentOffset: 3 }, { start: 2, end: 3 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆가가${marker}`, { baseOffset: 3, extentOffset: 3 }, { start: 2, end: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: '가', base: 1 }));
    expect(result.shouldSyncDom).toBe(false);
  });

  it('drops revived committed text that gets prepended to an active japanese composition', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '穴はな';
    element.setSelectionRange(0, 1);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '穴はな', base: 0 }),
      createTextEditingValue(`◆はな${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆はな${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'はな', base: 0 }));
    expect(result.shouldSyncDom).toBe(true);
  });

  it('broadens active japanese composition when IME duplicates the committed prefix for a wider candidate', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 3);
    element.value = 'はな花び';
    element.setSelectionRange(2, 3);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '花び', base: 2 }),
      createTextEditingValue(`◆はなび${marker}`, { baseOffset: 3, extentOffset: 4 }, { start: 3, end: 4 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆花び${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: '花び', base: 0 }));
    expect(result.shouldSyncDom).toBe(true);
  });

  it('keeps the committed prefix when an active japanese composition grows forward', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '花のにお';
    element.setSelectionRange(4, 4);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: 'にお', base: 2 }),
      createTextEditingValue(`◆花のに${marker}`, { baseOffset: 4, extentOffset: 4 }, { start: 3, end: 4 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆花のにお${marker}`, { baseOffset: 5, extentOffset: 5 }, { start: 3, end: 5 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'にお', base: 2 }));
    expect(result.shouldSyncDom).toBe(false);
  });

  it('keeps committed text outside the active composition when the caret is collapsed', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '日本かんk';
    element.setSelectionRange(5, 5);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: 'かんk', base: 2 }),
      createTextEditingValue(`◆日本かn${marker}`, { baseOffset: 5, extentOffset: 5 }, { start: 3, end: 5 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆日本かんk${marker}`, { baseOffset: 6, extentOffset: 6 }, { start: 3, end: 6 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'かんk', base: 2 }));
    expect(result.shouldSyncDom).toBe(false);
  });

  it('keeps the committed prefix when kana input replaces an in-progress kanji candidate', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '花のはなび';
    element.setSelectionRange(5, 5);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: 'はなび', base: 2 }),
      createTextEditingValue(`◆花の花b${marker}`, { baseOffset: 5, extentOffset: 5 }, { start: 3, end: 5 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆花のはなび${marker}`, { baseOffset: 6, extentOffset: 6 }, { start: 3, end: 6 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'はなび', base: 2 }));
    expect(result.shouldSyncDom).toBe(false);
  });

  it('keeps stripping revived text after the composition selection moves forward', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '穴はな';
    element.setSelectionRange(1, 3);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '穴はな', base: 0 }),
      createTextEditingValue(`◆はな${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆はな${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'はな', base: 0 }));
    expect(result.shouldSyncDom).toBe(true);
  });

  it('drops revived committed text after compositionend before commit', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '穴はな';
    element.setSelectionRange(3, 3);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({}),
      createTextEditingValue(`◆はな${marker}`, { baseOffset: 3, extentOffset: 3 }, { start: 1, end: 3 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆はな${marker}`, { baseOffset: 3, extentOffset: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({}));
    expect(result.shouldSyncDom).toBe(true);
  });

  it('preserves selected text while japanese reconversion emits a placeholder space', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = ' ';
    element.setSelectionRange(0, 1);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: ' ', base: 0, replaceStart: 0, replaceEnd: 2 }),
      createTextEditingValue(`◆はな${marker}`, { baseOffset: 3, extentOffset: 3 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆はな${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'はな', base: 0, replaceStart: 0, replaceEnd: 2 }));
    expect(result.shouldSyncDom).toBe(true);
  });

  it('keeps the existing japanese reconversion text when the placeholder collapses to empty', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = '';
    element.setSelectionRange(0, 0);

    const result = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: '', base: 0, replaceStart: 0, replaceEnd: 2 }),
      createTextEditingValue(`◆はな${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 }),
    );

    expect(result.value).toEqual(createTextEditingValue(`◆はな${marker}`, { baseOffset: 3, extentOffset: 3 }, { start: 1, end: 3 }));
    expect(result.tracking).toEqual(createCompositionTracking({ active: true, text: 'はな', base: 0, replaceStart: 0, replaceEnd: 2 }));
    expect(result.shouldSyncDom).toBe(true);
  });

  it('treats reconversion text over a selected range as a replacement instead of a fresh insertion', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 2);
    element.value = 'はなび';
    element.setSelectionRange(0, 2);

    const oldValue = createTextEditingValue(`◆はな${marker}`, { baseOffset: 3, extentOffset: 3 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', 'はなび');
    const readResult = readDomTextEditingValue(
      element,
      createCompositionTracking({ active: true, text: 'はなび', base: 0, replaceStart: 0, replaceEnd: 2 }),
      oldValue,
    );

    const deltas = inferSyntheticTextEditingDeltas(oldValue, readResult.value, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionUpdate', text: 'はなび', replaceLength: 2 }]);
    expect(result.state.currentValue).toEqual(
      createTextEditingValue(`◆はなび${marker}`, { baseOffset: 1, extentOffset: 4 }, { start: 1, end: 4 }),
    );
  });
});

describe('korean composition reduction', () => {
  it('emits compositionStart for a fresh composition insert', () => {
    const marker = positionMarker('node-1', 0);
    const oldValue = createTextEditingValue(`◆${marker}`, { baseOffset: 1, extentOffset: 1 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', 'はなび');
    const newValue = createTextEditingValue(`◆はなび${marker}`, { baseOffset: 1, extentOffset: 4 }, { start: 1, end: 4 });

    const deltas = inferSyntheticTextEditingDeltas(oldValue, newValue, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionStart', text: 'はなび' }]);
    expect(result.state.currentValue).toEqual(newValue);
  });

  it('emits compositionUpdate instead of committed input for insertCompositionText', () => {
    const marker = positionMarker('node-1', 1);
    const oldValue = createTextEditingValue(`◆ㅎ${marker}`, { baseOffset: 2, extentOffset: 2 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', '하');
    const newValue = createTextEditingValue(`◆하${marker}`, { baseOffset: 2, extentOffset: 2 }, { start: 1, end: 2 });

    const deltas = inferSyntheticTextEditingDeltas(oldValue, newValue, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionUpdate', text: '하', replaceLength: 1 }]);
    expect(result.state.currentValue).toEqual(newValue);
  });

  it('emits replaceLength when active composition expands backward over committed text', () => {
    const marker = positionMarker('node-1', 3);
    const oldValue = createTextEditingValue(`◆はなび${marker}`, { baseOffset: 3, extentOffset: 4 }, { start: 3, end: 4 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', '花び');
    const newValue = createTextEditingValue(`◆花び${marker}`, { baseOffset: 1, extentOffset: 3 }, { start: 1, end: 3 });

    const deltas = inferSyntheticTextEditingDeltas(oldValue, newValue, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionUpdate', text: '花び', replaceLength: 2 }]);
    expect(result.state.currentValue).toEqual(newValue);
  });

  it('does not emit replaceLength when an active japanese composition only grows forward', () => {
    const marker = positionMarker('node-1', 2);
    const oldValue = createTextEditingValue(`◆花のに${marker}`, { baseOffset: 4, extentOffset: 4 }, { start: 3, end: 4 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', 'にお');
    const newValue = createTextEditingValue(`◆花のにお${marker}`, { baseOffset: 5, extentOffset: 5 }, { start: 3, end: 5 });

    const deltas = inferSyntheticTextEditingDeltas(oldValue, newValue, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionUpdate', text: 'にお' }]);
    expect(result.state.currentValue).toEqual(newValue);
  });

  it('keeps the committed prefix when a japanese candidate rewrites the active composition text', () => {
    const marker = positionMarker('node-1', 2);
    const oldValue = createTextEditingValue(`◆花の花b${marker}`, { baseOffset: 5, extentOffset: 5 }, { start: 3, end: 5 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', 'はなび');
    const newValue = createTextEditingValue(`◆花のはなび${marker}`, { baseOffset: 6, extentOffset: 6 }, { start: 3, end: 6 });

    const deltas = inferSyntheticTextEditingDeltas(oldValue, newValue, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionUpdate', text: 'はなび' }]);
    expect(result.state.currentValue).toEqual(newValue);
  });

  it('treats duplicated japanese reconversion text as fresh composition after the cursor', () => {
    const marker = positionMarker('node-1', 2);
    const oldValue = createTextEditingValue(`漢字${marker}`, { baseOffset: 2, extentOffset: 2 });
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', '漢字はな');
    const newValue = createTextEditingValue(`漢字はな${marker}`, { baseOffset: 2, extentOffset: 4 }, { start: 2, end: 4 });

    const deltas = inferSyntheticTextEditingDeltas(oldValue, newValue, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(result.messages).toEqual([{ type: 'compositionStart', text: 'はな' }]);
    expect(result.state.currentValue).toEqual(newValue);
  });

  it('infers a composing range when IME selects the fresh candidate text', () => {
    const element = document.createElement('textarea');
    const marker = positionMarker('node-1', 0);
    element.value = 'はなび';
    element.setSelectionRange(0, 2);

    const oldValue = createTextEditingValue(`◆${marker}`, { baseOffset: 1, extentOffset: 1 });
    const readResult = readDomTextEditingValue(element, createCompositionTracking({ active: true, text: 'はなび' }), oldValue);
    const pending = createBeforeInputState(oldValue, 'insertCompositionText', 'はなび');
    const deltas = inferSyntheticTextEditingDeltas(oldValue, readResult.value, pending);
    const result = reduceEditingDeltas(createSyntheticTextInputState(oldValue), deltas);

    expect(readResult.value).toEqual(createTextEditingValue(`◆はなび${marker}`, { baseOffset: 1, extentOffset: 4 }, { start: 1, end: 4 }));
    expect(result.messages).toEqual([{ type: 'compositionStart', text: 'はなび' }]);
  });
});
