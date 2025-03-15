import { defineTokens } from '@pandacss/dev';

export const shadows = defineTokens.shadows({
  xsmall: {
    value: ['0px 1px 2px 0px rgba(20, 20, 21, 0.04), 0px 1px 2px 0px rgba(20, 20, 21, 0.04)'],
  },
  small: {
    value: ['0px 2px 6px 0px rgba(16, 24, 40, 0.06)'],
  },
  medium: {
    value: ['0px 6px 15px -2px rgba(16, 24, 40, 0.08)', '0px 6px 15px -2px rgba(16, 24, 40, 0.08)'],
  },
  large: {
    value: ['0px 8px 24px -3px rgba(16, 24, 40, 0.05)', '0px 8px 24px -3px rgba(16, 24, 40, 0.10)'],
  },
  xlarge: {
    value: ['0px 20px 40px -8px rgba(16, 24, 40, 0.05)', '0px 20px 40px -8px rgba(16, 24, 40, 0.10)'],
  },
  xxlarge: {
    value: ['0px 25px 60px -15px rgba(16, 24, 40, 0.12)', '0px 25px 60px -15px rgba(16, 24, 40, 0.20)'],
  },
});
