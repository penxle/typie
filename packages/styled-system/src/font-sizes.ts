import { defineTokens } from '@pandacss/dev';
import { rem } from './helpers';

export const fontSizes = defineTokens.fontSizes({
  ...rem(32),
});
