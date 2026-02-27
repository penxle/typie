import { defineKeyframes } from '@pandacss/dev';

export const keyframes = defineKeyframes({
  blink: {
    '0%, 100%': { opacity: '100' },
    '50%': { opacity: '0' },
  },
  pulse: {
    '0%, 100%': { opacity: '100' },
    '50%': { opacity: '40' },
  },
  'skeleton-typing-a': {
    '0%, 100%': { transform: 'scaleX(1)' },
    '50%': { transform: 'scaleX(0.7)' },
  },
  'skeleton-typing-b': {
    '0%, 100%': { transform: 'scaleX(1)' },
    '50%': { transform: 'scaleX(0.85)' },
  },
  'skeleton-typing-c': {
    '0%, 100%': { transform: 'scaleX(1)' },
    '50%': { transform: 'scaleX(0.6)' },
  },
  alarm: {
    '0%, 50%, 100%': { transform: 'rotate(0deg)' },
    '5%': { transform: 'rotate(12deg)' },
    '10%': { transform: 'rotate(-12deg)' },
    '15%': { transform: 'rotate(10deg)' },
    '20%': { transform: 'rotate(-10deg)' },
    '25%': { transform: 'rotate(8deg)' },
    '30%': { transform: 'rotate(-8deg)' },
    '35%': { transform: 'rotate(6deg)' },
    '40%': { transform: 'rotate(-6deg)' },
    '45%': { transform: 'rotate(4deg)' },
  },
});
