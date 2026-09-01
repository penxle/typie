import { describe, expect, it } from 'vitest';
import { computeRulerMarks } from './recent-edit-marks';

const regions = (...rs: { page: number; y: number; h: number; kind: 'added' | 'modified' | 'deleted' }[]) =>
  rs.map((r) => ({ page_idx: r.page, y: r.y, height: r.h, kind: r.kind }));

describe('computeRulerMarks', () => {
  it('maps a region to content-proportional track position', () => {
    const marks = computeRulerMarks(regions({ page: 0, y: 100, h: 50, kind: 'added' }), [0], 1, 1000, 500, 2);
    expect(marks).toHaveLength(1);
    expect(marks[0].top).toBeCloseTo(2 + (100 / 1000) * 500);
    expect(marks[0].height).toBeCloseTo((50 / 1000) * 500);
    expect(marks[0].kind).toBe('added');
  });

  it('applies page offset and zoom', () => {
    const marks = computeRulerMarks(regions({ page: 1, y: 10, h: 10, kind: 'modified' }), [0, 600], 2, 2000, 500, 2);
    expect(marks[0].top).toBeCloseTo(2 + ((600 + 20) / 2000) * 500);
  });

  it('scales mark height by zoom', () => {
    const marks = computeRulerMarks(regions({ page: 1, y: 10, h: 50, kind: 'modified' }), [0, 600], 2, 2000, 500, 2);
    expect(marks[0].top).toBeCloseTo(2 + ((600 + 20) / 2000) * 500);
    expect(marks[0].height).toBeCloseTo(((50 * 2) / 2000) * 500);
  });

  it('enforces minimum mark height', () => {
    const marks = computeRulerMarks(regions({ page: 0, y: 0, h: 1, kind: 'added' }), [0], 1, 100_000, 500, 2);
    expect(marks[0].height).toBe(3);
  });

  it('merges adjacent same-kind marks and keeps kinds separate', () => {
    const marks = computeRulerMarks(
      regions(
        { page: 0, y: 100, h: 50, kind: 'added' },
        { page: 0, y: 150, h: 50, kind: 'added' },
        { page: 0, y: 150, h: 50, kind: 'modified' },
      ),
      [0],
      1,
      1000,
      500,
      2,
    );
    expect(marks.filter((m) => m.kind === 'added')).toHaveLength(1);
    expect(marks.filter((m) => m.kind === 'modified')).toHaveLength(1);
  });

  it('spans the union of merged marks', () => {
    const marks = computeRulerMarks(
      regions(
        { page: 0, y: 100, h: 60, kind: 'added' },
        { page: 0, y: 140, h: 40, kind: 'added' },
        { page: 0, y: 150, h: 10, kind: 'added' },
      ),
      [0],
      1,
      1000,
      500,
      2,
    );
    expect(marks).toHaveLength(1);
    expect(marks[0].top).toBe(52);
    expect(marks[0].height).toBe(40);
  });

  it('merges across a gap up to the threshold and splits beyond it', () => {
    const within = computeRulerMarks(
      regions({ page: 0, y: 100, h: 40, kind: 'added' }, { page: 0, y: 142, h: 40, kind: 'added' }),
      [0],
      1,
      1000,
      500,
      2,
    );
    expect(within).toHaveLength(1);
    expect(within[0].height).toBe(41);

    const beyond = computeRulerMarks(
      regions({ page: 0, y: 100, h: 40, kind: 'added' }, { page: 0, y: 144, h: 40, kind: 'added' }),
      [0],
      1,
      1000,
      500,
      2,
    );
    expect(beyond).toHaveLength(2);
    expect(beyond[0].height).toBe(20);
  });

  it('renders deleted as fixed-height marker sorted last', () => {
    const marks = computeRulerMarks(
      regions({ page: 0, y: 500, h: 0, kind: 'deleted' }, { page: 0, y: 100, h: 50, kind: 'added' }),
      [0],
      1,
      1000,
      500,
      2,
    );
    expect(marks.at(-1)?.kind).toBe('deleted');
    expect(marks.at(-1)?.height).toBe(2);
  });

  it('skips regions on unmeasured pages', () => {
    const marks = computeRulerMarks(regions({ page: 3, y: 0, h: 10, kind: 'added' }), [0], 1, 1000, 500, 2);
    expect(marks).toHaveLength(0);
  });

  it('skips regions whose page offset is not a number', () => {
    const marks = computeRulerMarks(regions({ page: 0, y: 0, h: 10, kind: 'added' }), [NaN], 1, 1000, 500, 2);
    expect(marks).toHaveLength(0);
  });

  it('nudges a deleted marker flush against an overlapping bar on its nearer edge', () => {
    const below = computeRulerMarks(
      regions({ page: 0, y: 100, h: 2, kind: 'added' }, { page: 0, y: 102, h: 0, kind: 'deleted' }),
      [0],
      1,
      1000,
      500,
      2,
    );
    const bar = below[0];
    expect(below.at(-1)?.top).toBeCloseTo(bar.top + bar.height);

    const above = computeRulerMarks(
      regions({ page: 0, y: 102, h: 50, kind: 'added' }, { page: 0, y: 102, h: 0, kind: 'deleted' }),
      [0],
      1,
      1000,
      500,
      2,
    );
    expect(above.at(-1)?.top).toBeCloseTo(above[0].top - 2);
  });
});
