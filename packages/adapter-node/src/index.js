import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { build } from 'tsup';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const wasmPlugin = (outputDir) => ({
  name: 'wasm',
  setup(build) {
    const wasmOutputs = new Map();

    build.onResolve({ filter: /\.wasm$/ }, async (args) => {
      const wasmPath = path.resolve(args.resolveDir, args.path);
      return {
        path: wasmPath,
        namespace: 'wasm-loader',
      };
    });

    build.onLoad({ filter: /.*/, namespace: 'wasm-loader' }, async (args) => {
      const wasmBuffer = await fs.readFile(args.path);
      const hash = crypto.createHash('md5').update(wasmBuffer).digest('hex').slice(0, 8);
      const baseName = path.basename(args.path, '.wasm');
      const fileName = `${baseName}-${hash}.wasm`;

      wasmOutputs.set(args.path, fileName);

      const chunksDir = path.join(outputDir, 'chunks');
      await fs.mkdir(chunksDir, { recursive: true });
      await fs.writeFile(path.join(chunksDir, fileName), wasmBuffer);

      return {
        contents: `
          import { readFileSync } from 'node:fs';
          import { dirname, join } from 'node:path';
          import { fileURLToPath } from 'node:url';

          const __filename = fileURLToPath(import.meta.url);
          const __dirname = dirname(__filename);
          const wasmPath = join(__dirname, 'chunks', '${fileName}');
          const wasmBuffer = readFileSync(wasmPath);
          const wasmModule = new WebAssembly.Module(wasmBuffer);
          const wasmInstance = new WebAssembly.Instance(wasmModule, {});
          export default wasmInstance.exports;
          export const memory = wasmInstance.exports.memory;
        `,
        loader: 'js',
      };
    });
  },
});

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

      await fs.writeFile(
        path.join(tmp, 'index.js'),
        `
          import { serve } from './serve.js';
          import { Server } from './server/index.js';
          import { manifest, prerendered } from './server/manifest.js';

          await serve({ Server, manifest, prerendered });
        `,
      );

      builder.copy(path.join(__dirname, 'serve.js'), path.join(tmp, 'serve.js'));

      await build({
        entry: [path.join(tmp, 'index.js')],
        outDir: out,

        format: 'esm',
        target: 'esnext',

        esbuildPlugins: [wasmPlugin(out)],

        esbuildOptions: (options) => {
          options.chunkNames = 'chunks/[name]-[hash]';
        },
      });
    },
  };
};
