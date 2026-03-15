import { promisify } from 'node:util';
import { constants, deflateSync, gzipSync, zstdCompress } from 'node:zlib';
import { createMiddleware } from 'hono/factory';
import { COMPRESSIBLE_CONTENT_TYPE_REGEX } from 'hono/utils/compress';
import { match } from 'ts-pattern';

const zstdCompressAsync = promisify(zstdCompress);

const ENCODINGS = ['zstd', 'gzip', 'deflate'] as const;

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
      const bytes = await c.res.bytes();
      const compressed = await match(encoding)
        .with('zstd', () => zstdCompressAsync(bytes, { params: { [constants.ZSTD_c_compressionLevel]: 19 } }))
        .with('gzip', () => gzipSync(bytes, { level: 6 }))
        .with('deflate', () => deflateSync(bytes, { level: 6 }))
        .exhaustive();

      c.res = new Response(new Uint8Array(compressed), c.res);

      c.res.headers.set('Content-Encoding', encoding);
      c.res.headers.delete('Content-Length');
      c.res.headers.delete('Content-Range');

      const vary = c.res.headers.get('Vary');
      c.res.headers.set('Vary', vary && !vary.includes('Accept-Encoding') ? `${vary}, Accept-Encoding` : 'Accept-Encoding');
    } catch {
      // pass
    }
  });
