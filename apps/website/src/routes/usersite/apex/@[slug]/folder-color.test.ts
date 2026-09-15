import { describe, expect, it } from 'vitest';
import { FOLDER_COLORS, folderColor, folderCoverStyle } from './folder-color';

describe('folderColor', () => {
  it('is stable for the same id', () => {
    expect(folderColor('E0000000000000000000001')).toBe(folderColor('E0000000000000000000001'));
  });

  it('spreads different ids across the palette', () => {
    const ids = Array.from({ length: 200 }, (_, i) => `E${i.toString().padStart(22, '0')}`);
    const used = new Set(ids.map((id) => folderColor(id)));
    expect(used.size).toBe(FOLDER_COLORS.length);
  });

  it('maps every id to a cover style', () => {
    for (const id of ['a', 'b', 'c', 'd', 'e', 'f', 'g']) {
      const style = folderCoverStyle(id);
      expect(style.backgroundColor).toBe(`palette.${folderColor(id)}/20`);
      expect(style.color).toBe(`palette.${folderColor(id)}`);
    }
  });
});
