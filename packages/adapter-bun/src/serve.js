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

      if (await file.exists()) {
        return c.body(file, {
          headers: {
            'cache-control': immutable ? 'public, max-age=31536000, immutable' : 'public, max-age=0, must-revalidate',
          },
        });
      } else {
        return c.text('Not Found', {
          status: 404,
          headers: {
            'cache-control': 'no-store',
          },
        });
      }
    }

    if (c.req.path in prerendered) {
      const filePath = path.join(basePath, 'assets', prerendered[c.req.path]);
      const file = Bun.file(filePath);

      if (await file.exists()) {
        return c.body(file, {
          headers: {
            'cache-control': 'public, max-age=0, must-revalidate',
          },
        });
      } else {
        return c.text('Not Found', {
          status: 404,
          headers: {
            'cache-control': 'no-store',
          },
        });
      }
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
      if (response.ok) {
        response.headers.set('cache-control', 'private, no-cache');
      } else {
        response.headers.set('cache-control', 'no-store');
      }
    }

    return response;
  });

  Bun.serve({
    fetch: app.fetch,
    error: (err) => {
      if (err.code === 'ENOENT') {
        return new Response('Not Found', {
          status: 404,
          headers: {
            'cache-control': 'no-store',
          },
        });
      }

      return new Response('Internal Server Error', {
        status: 500,
        headers: {
          'cache-control': 'no-store',
        },
      });
    },
    port: 3000,
    idleTimeout: 0,
  });
};
