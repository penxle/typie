import { Readable } from 'node:stream';
import { constants, createBrotliCompress, createDeflate, createGzip, createZstdCompress } from 'node:zlib';
import { createMiddleware } from 'hono/factory';
import { COMPRESSIBLE_CONTENT_TYPE_REGEX } from 'hono/utils/compress';
import type { ReadableStream as NodeReadableStream } from 'node:stream/web';

const ENCODINGS = ['zstd', 'br', 'gzip', 'deflate'] as const;

function createCompressionStream(encoding: string) {
  switch (encoding) {
    case 'zstd': {
      return createZstdCompress({ params: { [constants.ZSTD_c_compressionLevel]: 3 } });
    }
    case 'br': {
      return createBrotliCompress({ params: { [constants.BROTLI_PARAM_QUALITY]: 4 } });
    }
    case 'gzip': {
      return createGzip({ level: 6 });
    }
    case 'deflate': {
      return createDeflate({ level: 6 });
    }
    default: {
      throw new Error(`Unknown encoding: ${encoding}`);
    }
  }
}

export const compression = () =>
  createMiddleware(async (c, next) => {
    await next();

    if (
      c.req.method === 'HEAD' ||
      !c.res.body ||
      c.res.headers.get('Content-Encoding') ||
      c.res.headers.get('Cache-Control')?.includes('no-transform')
    )
      return;

    const contentType = c.res.headers.get('Content-Type');
    if (contentType && !COMPRESSIBLE_CONTENT_TYPE_REGEX.test(contentType)) return;

    const contentLength = c.res.headers.get('Content-Length');
    if (contentLength && +contentLength < 1024) return;

    const acceptEncoding = c.req.header('Accept-Encoding');
    if (!acceptEncoding) return;

    const encoding = ENCODINGS.find((enc) => acceptEncoding.includes(enc));
    if (!encoding) return;

    try {
      const nodeReadable = Readable.fromWeb(c.res.body as NodeReadableStream);
      const compressionStream = createCompressionStream(encoding);
      nodeReadable.pipe(compressionStream);

      c.res = new Response(Readable.toWeb(compressionStream) as unknown as ReadableStream, c.res);

      c.res.headers.set('Content-Encoding', encoding);
      c.res.headers.delete('Content-Length');
      c.res.headers.delete('Content-Range');

      const vary = c.res.headers.get('Vary');
      c.res.headers.set('Vary', vary && !vary.includes('Accept-Encoding') ? `${vary}, Accept-Encoding` : 'Accept-Encoding');
    } catch {
      // pass
    }
  });
