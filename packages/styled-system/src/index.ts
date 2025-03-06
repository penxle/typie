import { definePreset } from '@pandacss/dev';
import { breakpoints } from './breakpoints';
import { conditions } from './conditions';
import { globalCss, globalFontface, globalVars } from './global';
import { keyframes } from './key-frames';
import { semanticTokens } from './semantic-tokens';
import { textStyles } from './text-styles';
import { tokens } from './tokens';
import { utilities } from './utilities';

export * as hooks from './hooks';

export const preset = definePreset({
  name: '@glitter/styled-system/preset',
  presets: ['@pandacss/preset-base'],

  theme: {
    breakpoints,
    tokens,
    semanticTokens,
    textStyles,
    keyframes,
  },

  conditions,
  utilities,

  globalCss,
  globalFontface,
  globalVars,
});
