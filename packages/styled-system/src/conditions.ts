export const conditions = {
  extend: {
    enabled: '&:is(:enabled, a[aria-disabled="false"])',
    disabled: '&:is(:disabled, [aria-disabled="true"])',
    hover: ['@media (hover: hover) and (pointer: fine)', '&:hover:not([aria-pressed="true"])'],
    groupHover: ['@media (hover: hover) and (pointer: fine)', '.group:hover &'],
    supportHover: ['@media (hover: hover) and (pointer: fine)', '&:hover'],
    active: ['@media (hover: hover) and (pointer: fine)', '&:active'],
    dark: '[data-theme="dark"] &',
    hoverAfter: ['@media (hover: hover) and (pointer: fine)', '&:hover::after'],
    groupSelected: '.group:is([aria-selected=true], [data-selected]) &',
  },
};
