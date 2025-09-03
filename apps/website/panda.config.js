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
      keyframes: {
        diceRoll: {
          '0%': { transform: 'rotate(0deg) scale(1)' },
          '25%': { transform: 'rotate(180deg) scale(1.2)' },
          '50%': { transform: 'rotate(360deg) scale(1.1)' },
          '75%': { transform: 'rotate(540deg) scale(1.05)' },
          '100%': { transform: 'rotate(720deg) scale(1)' },
        },
        pulse: {
          '0%, 100%': { opacity: '60' },
          '50%': { opacity: '100' },
        },
      },
    },
  },
});
