import { defineTokens } from '@pandacss/dev';
import { px } from './helpers';

export const radii = defineTokens.radii({
  ...px(16),
  full: { value: '50%' },
});
