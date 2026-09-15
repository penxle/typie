import { describe, expect, it } from 'vitest';
import { folderCardBlocks } from './folder-card';

describe('folderCardBlocks', () => {
  it('shows both blocks when the document has a parent folder and neighbors', () => {
    expect(folderCardBlocks({ folder: true, prev: true, next: false })).toEqual({ folder: true, siblings: true });
  });

  it('shows only the folder block when there are no neighbors', () => {
    expect(folderCardBlocks({ folder: true, prev: false, next: false })).toEqual({ folder: true, siblings: false });
  });

  it('shows only the siblings block for a root document with neighbors', () => {
    expect(folderCardBlocks({ folder: false, prev: false, next: true })).toEqual({ folder: false, siblings: true });
  });

  it('shows nothing for a lone root document', () => {
    expect(folderCardBlocks({ folder: false, prev: false, next: false })).toEqual({ folder: false, siblings: false });
  });
});
