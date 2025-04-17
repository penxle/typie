import { defineKeyframes } from '@pandacss/dev';

export const keyframes = defineKeyframes({
  blink: {
    '0%, 100%': { opacity: '100' },
    '50%': { opacity: '0' },
  },
});
