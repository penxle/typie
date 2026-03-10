import { defineTokens } from '@pandacss/dev';

export const gradients = defineTokens.gradients({
  grid: {
    value:
      'repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size, 30px) - var(--grid-line-thickness, 1px)), var(--grid-line-color) calc(var(--grid-size, 30px) - var(--grid-line-thickness, 1px)), var(--grid-line-color) var(--grid-size, 30px)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size, 30px) - var(--grid-line-thickness, 1px)), var(--grid-line-color) calc(var(--grid-size, 30px) - var(--grid-line-thickness, 1px)), var(--grid-line-color) var(--grid-size, 30px)), repeating-linear-gradient(0deg, transparent, transparent calc(var(--grid-size, 30px) / 2 - var(--grid-line-thickness, 1px)), var(--grid-cross-line-color) calc(var(--grid-size, 30px) / 2 - var(--grid-line-thickness, 1px)), var(--grid-cross-line-color) calc(var(--grid-size, 30px) / 2), transparent calc(var(--grid-size, 30px) / 2), transparent var(--grid-size, 30px)), repeating-linear-gradient(90deg, transparent, transparent calc(var(--grid-size, 30px) / 2 - var(--grid-line-thickness, 1px)), var(--grid-cross-line-color) calc(var(--grid-size, 30px) / 2 - var(--grid-line-thickness, 1px)), var(--grid-cross-line-color) calc(var(--grid-size, 30px) / 2), transparent calc(var(--grid-size, 30px) / 2), transparent var(--grid-size, 30px))',
  },
});
