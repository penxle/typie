import { sveltekit } from '@sveltejs/kit/vite';
import { svg } from '@typie/lib/vite';
import { sark } from '@typie/sark/vite';
import { FileSystemIconLoader } from 'unplugin-icons/loaders';
import icons from 'unplugin-icons/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  clearScreen: false,
  plugins: [
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore type mismatch
    svg(),
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore type mismatch
    icons({
      scale: 1,
      compiler: 'svelte',
      customCollections: {
        typie: FileSystemIconLoader('./src/icons'),
      },
    }),
    // eslint-disable-next-line @typescript-eslint/ban-ts-comment
    // @ts-ignore type mismatch
    sark(),
    sveltekit(),
  ],
  server: {
    port: 4000,
    strictPort: true,
  },
});
