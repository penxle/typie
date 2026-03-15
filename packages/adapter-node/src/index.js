import fs from 'node:fs/promises';
import path from 'node:path';
import { rolldown } from 'rolldown';

/**
 * @returns {import("@sveltejs/kit").Adapter}
 */
export const node = () => {
  return {
    name: '@typie/adapter-node',
    adapt: async (builder) => {
      const tmp = builder.getBuildDirectory('adapter-node');
      const out = 'dist';

      builder.rimraf(tmp);
      builder.rimraf(out);

      builder.mkdirp(tmp);
      builder.mkdirp(out);

      const prerendered = {};
      for (const [p, { file }] of builder.prerendered.pages.entries()) {
        prerendered[p] = file;
      }

      builder.log.info('Copying assets');
      builder.writeClient(path.join(out, 'assets'));
      builder.writePrerendered(path.join(out, 'assets'));

      builder.log.info('Building server');
      builder.writeServer(path.join(tmp, 'server'));

      await fs.appendFile(path.join(tmp, 'server/manifest.js'), `export const prerendered = ${JSON.stringify(prerendered)};`);

      builder.copy(path.join(import.meta.dirname, 'serve.js'), path.join(tmp, 'serve.js'));

      await fs.writeFile(
        path.join(tmp, 'index.js'),
        `
          import { serve } from './serve.js';
          import { Server } from './server/index.js';
          import { manifest, prerendered } from './server/manifest.js';

          await serve({ Server, manifest, prerendered });
        `,
      );

      const bundle = await rolldown({
        input: path.join(tmp, 'index.js'),
        platform: 'node',
        resolve: {
          conditionNames: ['node', 'import'],
        },
      });

      await bundle.write({
        dir: out,
        format: 'esm',
        chunkFileNames: 'chunks/[name]-[hash].js',
      });
    },
  };
};
