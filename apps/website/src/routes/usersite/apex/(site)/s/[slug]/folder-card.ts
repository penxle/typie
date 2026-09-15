export const folderCardBlocks = (input: { folder: boolean; prev: boolean; next: boolean }): { folder: boolean; siblings: boolean } => ({
  folder: input.folder,
  siblings: input.prev || input.next,
});
