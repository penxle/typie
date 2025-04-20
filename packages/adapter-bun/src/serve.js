import path from 'node:path';
import { getClientAddress } from '@typie/lib';
import { Hono } from 'hono';
import { compress } from 'hono-compress';

/**
 * @typedef {Object} ServeParams
 * @property {typeof import('@sveltejs/kit').Server} Server
 * @property {import('@sveltejs/kit').SSRManifest} manifest
 * @property {Record<string, string>} prerendered
 */

/**
 * @param {ServeParams} params
 */

export const serve = async ({ Server, manifest, prerendered }) => {
  const basePath = path.dirname(Bun.main);

  const sveltekit = new Server(manifest);
  await sveltekit.init({ env: process.env });

  const app = new Hono();

  app.use('*', compress());

  app.get('/healthz', (c) => c.json({ '*': true }));

  app.all('*', async (c) => {
    const relativePath = c.req.path.slice(1);
    if (manifest.assets.has(relativePath) || relativePath.startsWith(manifest.appPath)) {
      const immutable = relativePath.startsWith(`${manifest.appPath}/immutable`);
      const filePath = path.join(basePath, 'assets', relativePath);
      const file = Bun.file(filePath);

      return new Response(file, {
        headers: {
          'cache-control': immutable ? 'public, max-age=31536000, immutable' : 'public, max-age=0, must-revalidate',
          'content-type': file.type,
          'content-length': file.size,
        },
      });
    }

    if (c.req.path in prerendered) {
      const filePath = path.join(basePath, 'assets', prerendered[c.req.path]);
      const file = Bun.file(filePath);

      return new Response(file, {
        headers: {
          'cache-control': 'public, max-age=0, must-revalidate',
          'content-type': file.type,
          'content-length': file.size,
        },
      });
    }

    const url = new URL(c.req.url);
    url.protocol = process.env.NODE_ENV === 'production' ? 'https:' : 'http:';

    const request = new Request(url, c.req.raw);
    const response = await sveltekit.respond(request, {
      getClientAddress: () => {
        return getClientAddress(c);
      },
    });

    if (response.headers.get('cache-control') === null) {
      response.headers.set('cache-control', 'private, no-cache');
    }

    return response;
  });

  app.onError((_, c) => {
    return c.text('Internal Server Error', { status: 500 });
  });

  Bun.serve({
    fetch: app.fetch,
    error: (err) => {
      if (err.code === 'ENOENT') {
        return new Response('Not Found', {
          status: 404,
        });
      }

      return new Response('Internal Server Error', {
        status: 500,
      });
    },
    port: 3000,
    idleTimeout: 0,
  });
};
