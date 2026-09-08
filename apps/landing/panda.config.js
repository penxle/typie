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

  theme: {
    extend: {
      tokens: {
        fonts: {
          ui: { value: 'WantedSans' },
        },
      },
    },
  },

  globalFontface: {
    WantedSans: {
      src: 'url("https://cdn.typie.net/fonts/WantedSans-Variable.woff2") format("woff2-variations")',
      fontStyle: 'normal',
      fontWeight: '400 1000',
      fontDisplay: 'swap',
    },
  },

  globalCss: {
    extend: {
      html: {
        backgroundColor: 'surface.canvas',
        wordBreak: 'keep-all',
      },
      ':focus-visible': {
        outline: '2px solid token(colors.accent.default)',
        outlineOffset: '-1px',
      },
    },
  },
});
