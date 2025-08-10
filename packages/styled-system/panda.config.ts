import { defineConfig } from '@pandacss/dev';
import { preset } from './src';

export default defineConfig({
  outExtension: 'js',

  eject: true,
  presets: [preset],

  strictPropertyValues: true,
  strictTokens: true,

  separator: '-',
  hash: true,
  minify: true,
});
