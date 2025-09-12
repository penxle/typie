export const compressZstd = async (data: Uint8Array): Promise<Uint8Array> => {
  const buffer = await Bun.zstdCompress(data, { level: 6 });
  return Uint8Array.from(buffer);
};

export const decompressZstd = async (data: Uint8Array): Promise<Uint8Array> => {
  const buffer = await Bun.zstdDecompress(data);
  return Uint8Array.from(buffer);
};
