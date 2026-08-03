import { describe, expect, it } from 'vitest';
import { initialCompositionTailState, resolveCompositionTail } from './composition-tail-resolver';

describe('resolveCompositionTail', () => {
  it('commits the active composition before inserting an appended Korean final key', () => {
    const keyed = resolveCompositionTail(initialCompositionTailState, { type: 'key_down', key: ' ' });
    const updated = resolveCompositionTail(keyed.state, { type: 'composition_continues' });
    const deferred = resolveCompositionTail(updated.state, {
      type: 'composition_edit',
      currentText: 'ㅠ',
      editText: 'ㅠ ',
      targetsCurrentComposition: true,
    });
    expect(deferred.effects).toEqual([{ type: 'defer_current_edit', generation: 1 }]);

    const ended = resolveCompositionTail(deferred.state, { type: 'composition_end' });
    expect(ended.effects).toEqual([{ type: 'commit_then_insert', generation: 1, text: ' ' }]);
  });

  it('applies a deferred Japanese continuation as a composition edit', () => {
    const keyed = resolveCompositionTail(initialCompositionTailState, { type: 'key_down', key: 'n' });
    const deferred = resolveCompositionTail(keyed.state, {
      type: 'composition_edit',
      currentText: 'にほ',
      editText: 'にほn',
      targetsCurrentComposition: true,
    });

    const continued = resolveCompositionTail(deferred.state, { type: 'composition_continues' });
    expect(continued.effects).toEqual([{ type: 'apply_deferred_edit', generation: 1 }]);
  });

  it('applies a deferred edit when no composition-end observation arrives', () => {
    const keyed = resolveCompositionTail(initialCompositionTailState, { type: 'key_down', key: 'n' });
    const deferred = resolveCompositionTail(keyed.state, {
      type: 'composition_edit',
      currentText: 'にほ',
      editText: 'にほn',
      targetsCurrentComposition: true,
    });

    const timedOut = resolveCompositionTail(deferred.state, { type: 'timeout', generation: 1 });
    expect(timedOut.effects).toEqual([{ type: 'apply_deferred_edit', generation: 1 }]);
  });

  it('ignores a stale timeout after reset discarded the deferred edit', () => {
    const keyed = resolveCompositionTail(initialCompositionTailState, { type: 'key_down', key: ' ' });
    const deferred = resolveCompositionTail(keyed.state, {
      type: 'composition_edit',
      currentText: 'ㅠ',
      editText: 'ㅠ ',
      targetsCurrentComposition: true,
    });
    const reset = resolveCompositionTail(deferred.state, { type: 'reset' });
    expect(reset.effects).toEqual([{ type: 'discard_deferred_edit', generation: 1 }]);

    const staleTimeout = resolveCompositionTail(reset.state, { type: 'timeout', generation: 1 });
    expect(staleTimeout.effects).toEqual([]);
  });
});
