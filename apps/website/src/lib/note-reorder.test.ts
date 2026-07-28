import { describe, expect, it } from 'vitest';
import { reorderedNoteIdsForDrag } from './note-reorder';
import type { NoteReorderGeometry } from './note-reorder';

const geometry = (entries: Record<string, NoteReorderGeometry>) => new Map(Object.entries(entries));

describe('reorderedNoteIdsForDrag', () => {
  it('does not swap at exactly half overlap with the next note', () => {
    const result = reorderedNoteIdsForDrag(
      ['dragged', 'next', 'tail'],
      'dragged',
      1,
      geometry({
        dragged: { top: 50, bottom: 150 },
        next: { top: 100, bottom: 200 },
        tail: { top: 200, bottom: 300 },
      }),
    );

    expect(result).toEqual(['dragged', 'next', 'tail']);
  });

  it('swaps with the next note once overlap is greater than half the smaller height', () => {
    const result = reorderedNoteIdsForDrag(
      ['dragged', 'next', 'tail'],
      'dragged',
      1,
      geometry({
        dragged: { top: 85, bottom: 145 },
        next: { top: 100, bottom: 300 },
        tail: { top: 300, bottom: 360 },
      }),
    );

    expect(result).toEqual(['next', 'dragged', 'tail']);
  });

  it('swaps with the previous note once overlap is greater than half the smaller height', () => {
    const result = reorderedNoteIdsForDrag(
      ['head', 'dragged', 'tail'],
      'dragged',
      -1,
      geometry({
        head: { top: 0, bottom: 100 },
        dragged: { top: 49, bottom: 149 },
        tail: { top: 200, bottom: 300 },
      }),
    );

    expect(result).toEqual(['dragged', 'head', 'tail']);
  });

  it('does not swap at exactly half overlap with the previous note', () => {
    const result = reorderedNoteIdsForDrag(
      ['head', 'dragged', 'tail'],
      'dragged',
      -1,
      geometry({
        head: { top: 0, bottom: 100 },
        dragged: { top: 50, bottom: 150 },
        tail: { top: 200, bottom: 300 },
      }),
    );

    expect(result).toEqual(['head', 'dragged', 'tail']);
  });

  it('swaps down after fully crossing the next note without overlap', () => {
    const result = reorderedNoteIdsForDrag(
      ['dragged', 'next', 'tail'],
      'dragged',
      1,
      geometry({
        dragged: { top: 200, bottom: 300 },
        next: { top: 100, bottom: 200 },
        tail: { top: 300, bottom: 400 },
      }),
    );

    expect(result).toEqual(['next', 'dragged', 'tail']);
  });

  it('swaps up after fully crossing the previous note without overlap', () => {
    const result = reorderedNoteIdsForDrag(
      ['head', 'dragged', 'tail'],
      'dragged',
      -1,
      geometry({
        head: { top: 100, bottom: 200 },
        dragged: { top: 0, bottom: 100 },
        tail: { top: 300, bottom: 400 },
      }),
    );

    expect(result).toEqual(['dragged', 'head', 'tail']);
  });

  it('does not reorder while stationary after a prior swap', () => {
    const result = reorderedNoteIdsForDrag(
      ['a', 'c', 'b', 'd'],
      'b',
      0,
      geometry({
        a: { top: 0, bottom: 50 },
        c: { top: 100, bottom: 150 },
        b: { top: 50, bottom: 100 },
        d: { top: 150, bottom: 200 },
      }),
    );

    expect(result).toEqual(['a', 'c', 'b', 'd']);
  });

  it('crosses multiple next notes in one update', () => {
    const result = reorderedNoteIdsForDrag(
      ['dragged', 'first', 'second', 'tail'],
      'dragged',
      1,
      geometry({
        dragged: { top: 175, bottom: 225 },
        first: { top: 50, bottom: 100 },
        second: { top: 100, bottom: 150 },
        tail: { top: 250, bottom: 300 },
      }),
    );

    expect(result).toEqual(['first', 'second', 'dragged', 'tail']);
  });

  it('crosses multiple previous notes in one update', () => {
    const result = reorderedNoteIdsForDrag(
      ['head', 'first', 'second', 'dragged', 'tail'],
      'dragged',
      -1,
      geometry({
        head: { top: 0, bottom: 50 },
        first: { top: 100, bottom: 150 },
        second: { top: 200, bottom: 250 },
        dragged: { top: 40, bottom: 90 },
        tail: { top: 300, bottom: 350 },
      }),
    );

    expect(result).toEqual(['head', 'dragged', 'first', 'second', 'tail']);
  });

  it('does not mutate the input note IDs', () => {
    const noteIds = ['dragged', 'next'];

    const result = reorderedNoteIdsForDrag(
      noteIds,
      'dragged',
      1,
      geometry({
        dragged: { top: 60, bottom: 160 },
        next: { top: 100, bottom: 200 },
      }),
    );

    expect(result).toEqual(['next', 'dragged']);
    expect(noteIds).toEqual(['dragged', 'next']);
  });

  it('returns null when the dragged note geometry is missing', () => {
    const result = reorderedNoteIdsForDrag(['a', 'b'], 'a', 1, geometry({ b: { top: 50, bottom: 100 } }));

    expect(result).toBeNull();
  });
});
