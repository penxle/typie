import { constants, createZstdCompress, createZstdDecompress } from 'node:zlib';

export const compressZstd = (data: Buffer): Promise<Buffer> => {
  return new Promise((resolve, reject) => {
    const compress = createZstdCompress({ params: { [constants.ZSTD_c_compressionLevel]: 22 } });
    const chunks: Buffer[] = [];

    compress.on('data', (chunk) => chunks.push(chunk));
    compress.on('end', () => resolve(Buffer.concat(chunks)));
    compress.on('error', reject);

    compress.write(data);
    compress.end();
  });
};

export const decompressZstd = (data: Buffer): Promise<Buffer> => {
  return new Promise((resolve, reject) => {
    const decompress = createZstdDecompress();
    const chunks: Buffer[] = [];

    decompress.on('data', (chunk) => chunks.push(chunk));
    decompress.on('end', () => resolve(Buffer.concat(chunks)));
    decompress.on('error', reject);

    decompress.write(data);
    decompress.end();
  });
};
