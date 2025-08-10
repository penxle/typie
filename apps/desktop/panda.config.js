import { defineConfig } from '@pandacss/dev';
import { preset } from '@typie/styled-system';

export default defineConfig({
  importMap: '@typie/styled-system',
  include: ['./src/**/*.{js,ts,svelte}', '../../packages/ui/src/**/*.{js,ts,svelte}'],

  eject: true,
  presets: [preset],

  separator: '-',
  hash: true,
  minify: true,
});
