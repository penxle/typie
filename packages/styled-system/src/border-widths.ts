import { defineTokens } from '@pandacss/dev';
import { rem } from './helpers';

export const borderWidths = defineTokens.borderWidths({
  ...rem(4),
});
