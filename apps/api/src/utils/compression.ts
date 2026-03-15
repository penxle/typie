import { promisify } from 'node:util';
import { constants, zstdCompress, zstdDecompress } from 'node:zlib';

const zstdCompressAsync = promisify(zstdCompress);
const zstdDecompressAsync = promisify(zstdDecompress);

export const compressZstd = async (data: Uint8Array): Promise<Uint8Array> => {
  const buffer = await zstdCompressAsync(data, {
    params: { [constants.ZSTD_c_compressionLevel]: 6 },
  });
  return new Uint8Array(buffer);
};

export const decompressZstd = async (data: Uint8Array): Promise<Uint8Array> => {
  const buffer = await zstdDecompressAsync(data);
  return new Uint8Array(buffer);
};
