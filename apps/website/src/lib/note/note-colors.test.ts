import { NOTE_COLORS } from '@typie/lib/catalogs';
import { describe, expect, it } from 'vitest';
import { noteColors } from './note-colors.ts';

describe('노트 색 카탈로그 대조', () => {
  it('카탈로그의 모든 색에 이름표와 색이 붙어 있다', () => {
    expect(noteColors.length).toBe(NOTE_COLORS.length);

    for (const value of NOTE_COLORS) {
      const option = noteColors.find((color) => color.value === value);
      expect(option?.label, value).toBeDefined();
      expect(option?.color, value).toBeDefined();
    }
  });
});
