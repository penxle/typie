import { defineTokens } from '@pandacss/dev';

export const shadows = defineTokens.shadows({
  small: {
    value: ['0 1px 2px {colors.shadow.default/3}', '0 2px 4px {colors.shadow.default/3}'],
  },
  medium: {
    value: ['0 1px 2px {colors.shadow.default/5}', '0 2px 4px {colors.shadow.default/5}', '0 4px 8px {colors.shadow.default/5}'],
  },
  large: {
    value: [
      '0 1px 2px {colors.shadow.default/7}',
      '0 2px 4px {colors.shadow.default/7}',
      '0 4px 8px {colors.shadow.default/7}',
      '0 8px 16px {colors.shadow.default/7}',
    ],
  },
});
