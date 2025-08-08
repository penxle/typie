import { defineTokens } from '@pandacss/dev';
import { rem } from './helpers';

export const spacing = defineTokens.spacing({
  ...rem(1200),

  '1/2': { value: '50%' },
});
