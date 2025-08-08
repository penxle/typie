import { definePreset } from '@pandacss/dev';
import { breakpoints } from './breakpoints';
import { conditions } from './conditions';
import { globalCss, globalFontface, globalVars } from './global';
import { keyframes } from './keyframes';
import { semanticTokens, tokens } from './tokens';
import { utilities } from './utilities';

export const preset = definePreset({
  name: '@typie/website',
  presets: ['@pandacss/preset-base'],

  theme: {
    breakpoints,
    tokens,
    semanticTokens,
    keyframes,
  },

  conditions,
  utilities,

  globalCss,
  globalFontface,
  globalVars,
});
