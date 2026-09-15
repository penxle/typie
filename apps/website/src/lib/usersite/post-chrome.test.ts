import { describe, expect, it } from 'vitest';
import { chromeHidden, readingProgress, titleSlot } from './post-chrome';

describe('titleSlot', () => {
  it('switches to the title once the title has passed the sticky bottom', () => {
    expect(titleSlot(52, 52)).toBe('title');
    expect(titleSlot(40, 52)).toBe('title');
  });

  it('keeps the eyebrow while the title is still visible', () => {
    expect(titleSlot(53, 52)).toBe('');
  });

  it('keeps the eyebrow when the title is not measured', () => {
    expect(titleSlot(undefined, 52)).toBe('');
  });
});

describe('readingProgress', () => {
  it('maps scroll position onto 0..1', () => {
    expect(readingProgress(0, 2000, 800)).toBe(0);
    expect(readingProgress(600, 2000, 800)).toBe(0.5);
    expect(readingProgress(1200, 2000, 800)).toBe(1);
  });

  it('clamps overscroll', () => {
    expect(readingProgress(-10, 2000, 800)).toBe(0);
    expect(readingProgress(5000, 2000, 800)).toBe(1);
  });

  it('is zero when the page does not scroll', () => {
    expect(readingProgress(0, 800, 800)).toBe(0);
    expect(readingProgress(0, 600, 800)).toBe(0);
  });
});

describe('chromeHidden', () => {
  it('hides only on a retreating post page below desktop', () => {
    expect(chromeHidden({ post: true, retreat: true, desktop: false })).toBe(true);
    expect(chromeHidden({ post: true, retreat: true, desktop: true })).toBe(false);
    expect(chromeHidden({ post: true, retreat: false, desktop: false })).toBe(false);
    expect(chromeHidden({ post: false, retreat: true, desktop: false })).toBe(false);
  });
});
