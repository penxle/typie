export const conditions = {
  extend: {
    enabled: '&:is(:enabled, a[aria-disabled="false"])',
    disabled: '&:is(:disabled, [aria-disabled="true"])',
    hover: '&:hover:not([aria-pressed="true"])',
    supportHover: ['@media (hover: hover) and (pointer: fine)', '&:hover'],
    dark: '[data-theme="dark"] &',
  },
};
