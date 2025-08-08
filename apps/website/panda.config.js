import { defineConfig } from '@pandacss/dev';
import { preset } from '@typie/styled-system';

const prod = process.env.NODE_ENV === 'production';

export default defineConfig({
  importMap: '@typie/styled-system',
  include: ['./src/**/*.{js,ts,svelte}'],

  eject: true,
  presets: [preset],

  separator: '-',
  hash: prod,
  minify: prod,
});
