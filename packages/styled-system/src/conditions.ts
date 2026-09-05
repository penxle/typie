import { variantConditions } from './conditions.generated';

const hoverGuard =
  ':not(:where(:focus-visible, [aria-pressed="true"], [aria-checked="true"], [aria-selected="true"], [aria-current]:not([aria-current="false"]), [aria-haspopup][aria-expanded="true"], [data-context-menu-open="true"]))';

export const conditions = {
  extend: {
    enabled: '&:is(:enabled, a[aria-disabled="false"])',
    disabled: '&:is(:disabled, [aria-disabled="true"])',
    hover: ['@media (hover: hover) and (pointer: fine)', `&:hover${hoverGuard}`],
    groupHover: ['@media (hover: hover) and (pointer: fine)', '.group:hover &'],
    supportHover: ['@media (hover: hover) and (pointer: fine)', `&:hover${hoverGuard}`],
    active: ['@media (hover: hover) and (pointer: fine)', '&:active'],
    hoverAfter: ['@media (hover: hover) and (pointer: fine)', '&:hover::after'],
    groupSelected: '.group:is([aria-selected=true], [data-selected]) &',

    dark: '[data-theme="dark"] &',
    ...variantConditions,
  },
};
