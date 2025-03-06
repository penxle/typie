import { defineKeyframes } from '@pandacss/dev';

export const keyframes = defineKeyframes({
  pulseFade: {
    '0%': { opacity: '80' },
    '50%': { opacity: '50' },
    '100%': { opacity: '80' },
  },
  pulseFadeDark: {
    '0%': { opacity: '40' },
    '50%': { opacity: '20' },
    '100%': { opacity: '40' },
  },
});
