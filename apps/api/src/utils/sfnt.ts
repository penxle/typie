export const sfntDirectoryLength = (data: Uint8Array): number | null => {
  if (data.length < 12) {
    return null;
  }
  const numTables = (data[4] << 8) | data[5];
  return 12 + 16 * numTables;
};

export const sfntHasTable = (data: Uint8Array, tag: string): boolean | null => {
  const directoryLength = sfntDirectoryLength(data);
  if (directoryLength === null || data.length < directoryLength) {
    return null;
  }
  for (let i = 12; i < directoryLength; i += 16) {
    if (String.fromCodePoint(data[i], data[i + 1], data[i + 2], data[i + 3]) === tag) {
      return true;
    }
  }
  return false;
};
