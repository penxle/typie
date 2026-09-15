import { describe, expect, it } from 'vitest';
import { folderCountLabel } from './folder-count';

describe('folderCountLabel', () => {
  it('joins the non-zero direct child counts with a middle dot', () => {
    expect(folderCountLabel(1, 4)).toBe('폴더 1개 · 글 4개');
    expect(folderCountLabel(1, 0)).toBe('폴더 1개');
    expect(folderCountLabel(0, 4)).toBe('글 4개');
    expect(folderCountLabel(0, 0)).toBe('');
  });
});
