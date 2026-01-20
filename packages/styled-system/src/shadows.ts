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
  card: {
    value: ['0 3px 6px -2px {colors.shadow.default/3}', '0 1px 1px {colors.shadow.default/5}'],
  },
  modern: {
    value: ['0 4px 6px -1px rgba(0, 0, 0, 0.1)', '0 2px 4px -2px rgba(0, 0, 0, 0.1)'],
  },
  modernLg: {
    value: ['0 10px 15px -3px rgba(0, 0, 0, 0.1)', '0 4px 6px -4px rgba(0, 0, 0, 0.1)'],
  },
  modernXl: {
    value: ['0 20px 25px -5px rgba(0, 0, 0, 0.1)', '0 8px 10px -6px rgba(0, 0, 0, 0.1)'],
  },
  brandGlow: {
    value: '0 0 60px 20px rgba(209, 130, 57, 0.15)',
  },
  brandGlowLg: {
    value: '0 0 80px 30px rgba(209, 130, 57, 0.2)',
  },
  cardGlow: {
    value: '0 0 30px rgba(209, 130, 57, 0.2)',
  },
  cardGlowSubtle: {
    value: '0 0 20px rgba(209, 130, 57, 0.1)',
  },
});
