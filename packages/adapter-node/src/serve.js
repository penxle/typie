import fs from 'node:fs/promises';
import path from 'node:path';
import { serve as honoServe } from '@hono/node-server';
import { compression, getClientAddress } from '@typie/lib';
import { Hono } from 'hono';
import { getMimeType } from 'hono/utils/mime';

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
  const basePath = import.meta.dirname;

  const sveltekit = new Server(manifest);
  await sveltekit.init({ env: process.env });

  const app = new Hono();

  app.use('*', compression());

  app.get('/healthz', (c) => c.json({ '*': true }));

  app.all('*', async (c) => {
    const relativePath = c.req.path.slice(1);
    if (manifest.assets.has(relativePath) || relativePath.startsWith(manifest.appPath)) {
      const immutable = relativePath.startsWith(`${manifest.appPath}/immutable`);
      const filePath = path.join(basePath, 'assets', relativePath);

      let file;
      try {
        file = await fs.open(filePath, 'r');
        const stat = await file.stat();

        if (stat.size > 0) {
          const buffer = await file.readFile();
          const mime = getMimeType(filePath);

          return c.body(buffer, {
            headers: {
              'cache-control': immutable ? 'public, max-age=31536000, immutable' : 'public, max-age=0, must-revalidate',
              'content-type': mime || 'application/octet-stream',
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
      } finally {
        await file?.close();
      }
    }

    if (c.req.path in prerendered) {
      const filePath = path.join(basePath, 'assets', prerendered[c.req.path]);

      let file;
      try {
        file = await fs.open(filePath, 'r');
        const stat = await file.stat();

        if (stat.size > 0) {
          const buffer = await file.readFile();
          const mime = getMimeType(filePath);

          return c.body(buffer, {
            headers: {
              'cache-control': 'public, max-age=0, must-revalidate',
              'content-type': mime || 'application/octet-stream',
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
      } finally {
        await file?.close();
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

  honoServe({
    fetch: app.fetch,
    hostname: '0.0.0.0',
    port: 3000,
  });
};
