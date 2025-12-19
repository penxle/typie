export const conditions = {
  extend: {
    enabled: '&:is(:enabled, a[aria-disabled="false"])',
    disabled: '&:is(:disabled, [aria-disabled="true"])',
    hover: ['@media (hover: hover) and (pointer: fine)', '&:hover:not([aria-pressed="true"])'],
    groupHover: ['@media (hover: hover) and (pointer: fine)', '.group:hover &'],
    supportHover: ['@media (hover: hover) and (pointer: fine)', '&:hover'],
    active: ['@media (hover: hover) and (pointer: fine)', '&:active'],
    hoverAfter: ['@media (hover: hover) and (pointer: fine)', '&:hover::after'],
    groupSelected: '.group:is([aria-selected=true], [data-selected]) &',

    dark: '[data-theme="dark"] &',

    lightWhite: '[data-theme="light"][data-variant-light="white"] &',
    lightSnow: '[data-theme="light"][data-variant-light="snow"] &',
    lightButter: '[data-theme="light"][data-variant-light="butter"] &',
    lightPeach: '[data-theme="light"][data-variant-light="peach"] &',
    lightRose: '[data-theme="light"][data-variant-light="rose"] &',
    lightSand: '[data-theme="light"][data-variant-light="sand"] &',
    lightMint: '[data-theme="light"][data-variant-light="mint"] &',
    lightCaramel: '[data-theme="light"][data-variant-light="caramel"] &',

    darkBlack: '[data-theme="dark"][data-variant-dark="black"] &',
    darkCharcoal: '[data-theme="dark"][data-variant-dark="charcoal"] &',
    darkGraphite: '[data-theme="dark"][data-variant-dark="graphite"] &',
    darkMidnight: '[data-theme="dark"][data-variant-dark="midnight"] &',
    darkNavy: '[data-theme="dark"][data-variant-dark="navy"] &',
    darkObsidian: '[data-theme="dark"][data-variant-dark="obsidian"] &',
    darkStorm: '[data-theme="dark"][data-variant-dark="storm"] &',
    darkEspresso: '[data-theme="dark"][data-variant-dark="espresso"] &',
  },
};
