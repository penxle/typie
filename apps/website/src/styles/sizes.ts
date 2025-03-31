import { defineTokens } from '@pandacss/dev';
import { rem } from './helpers';

export const sizes = defineTokens.sizes({
  ...rem(1200),

  full: { value: '100%' },
  none: { value: 'none' },

  min: { value: 'min-content' },
  fit: { value: 'fit-content' },
  max: { value: 'max-content' },
});
