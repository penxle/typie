import { defineTokens } from '@pandacss/dev';

export const shadows = defineTokens.shadows({
  small: {
    value: ['0 1px 2px {colors.gray.950/3}', '0 2px 4px {colors.gray.950/3}'],
  },
  medium: {
    value: ['0 1px 2px {colors.gray.950/5}', '0 2px 4px {colors.gray.950/5}', '0 4px 8px {colors.gray.950/5}'],
  },
  large: {
    value: [
      '0 1px 2px {colors.gray.950/7}',
      '0 2px 4px {colors.gray.950/7}',
      '0 4px 8px {colors.gray.950/7}',
      '0 8px 16px {colors.gray.950/7}',
    ],
  },
});
