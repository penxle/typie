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
  spin: {
    from: { transform: 'rotate(0deg)' },
    to: { transform: 'rotate(360deg)' },
  },
  'rise-in': {
    from: { opacity: '0', transform: 'translateY(4px)' },
    to: { opacity: '100', transform: 'translateY(0)' },
  },
  shimmer: {
    from: { backgroundPosition: '200% 0' },
    to: { backgroundPosition: '-200% 0' },
  },
  breathe: {
    '0%, 100%': { transform: 'scale(0.94)', opacity: '85' },
    '50%': { transform: 'scale(1.05)', opacity: '100' },
  },
  'hue-drift': {
    '0%, 100%': { filter: 'hue-rotate(0deg) blur(4px)' },
    '50%': { filter: 'hue-rotate(-28deg) blur(4px)' },
  },
  // opacity 전용 유지 — transform·filter를 걸려면 스팬이 inline-block이어야 하는데, inline-block은 평문과
  // 줄바꿈 규칙이 달라 라이브 줄이 확정 평문으로 바뀌는 순간 문단 전체가 재배치된다(레이아웃 시프트).
  reveal: {
    from: { opacity: '0' },
    to: { opacity: '100' },
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
