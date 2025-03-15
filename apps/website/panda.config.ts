import { defineConfig } from '@pandacss/dev';
import { preset } from './src/styles';

const prod = process.env.NODE_ENV === 'production';

export default defineConfig({
  importMap: '$styled-system',

  include: ['./src/**/*.{js,ts,svelte}'],
  outExtension: 'js',

  eject: true,
  presets: [preset],

  strictPropertyValues: true,
  strictTokens: true,

  separator: '-',
  hash: prod,
  minify: prod,
});
