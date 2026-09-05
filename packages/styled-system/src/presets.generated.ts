export const LIGHT_VARIANTS = [
  'white',
  'catppuccin-latte',
  'flexoki',
  'everforest',
  'rose-pine-dawn',
  'cupcake',
  'sakura',
  'silk',
] as const;
export const DARK_VARIANTS = ['black', 'flexoki', 'rose-pine', 'catppuccin-mocha', 'nightfox', 'dracula', 'nord', 'everforest'] as const;

export type LightVariant = (typeof LIGHT_VARIANTS)[number];
export type DarkVariant = (typeof DARK_VARIANTS)[number];
export type ThemeVariant = `light-${LightVariant}` | `dark-${DarkVariant}`;

export const DEFAULT_LIGHT_VARIANT: LightVariant = 'white';
export const DEFAULT_DARK_VARIANT: DarkVariant = 'black';

export const VARIANT_LABELS: Record<ThemeVariant, string> = {
  'light-white': 'White',
  'light-catppuccin-latte': 'Catppuccin Latte',
  'light-flexoki': 'Flexoki Light',
  'light-everforest': 'Everforest Light',
  'light-rose-pine-dawn': 'Rose Pine Dawn',
  'light-cupcake': 'Cupcake',
  'light-sakura': 'Sakura',
  'light-silk': 'Silk Light',
  'dark-black': 'Black',
  'dark-flexoki': 'Flexoki Dark',
  'dark-rose-pine': 'Rose Pine',
  'dark-catppuccin-mocha': 'Catppuccin Mocha',
  'dark-nightfox': 'Nightfox',
  'dark-dracula': 'Dracula',
  'dark-nord': 'Nord',
  'dark-everforest': 'Everforest Dark',
};

export const VARIANT_CANVAS: Record<ThemeVariant, string> = {
  'light-white': '#f9fafd',
  'light-catppuccin-latte': '#e6e9ef',
  'light-flexoki': '#f2f0e5',
  'light-everforest': '#efebd4',
  'light-rose-pine-dawn': '#fffaf3',
  'light-cupcake': '#f8eeef',
  'light-sakura': '#f8e2e7',
  'light-silk': '#e6eeec',
  'dark-black': '#0a0b0e',
  'dark-flexoki': '#1c1b1a',
  'dark-rose-pine': '#1f1d2e',
  'dark-catppuccin-mocha': '#181825',
  'dark-nightfox': '#131a24',
  'dark-dracula': '#323443',
  'dark-nord': '#3b4252',
  'dark-everforest': '#232a2e',
};

export const VARIANT_SELECTION: Record<ThemeVariant, string> = {
  'light-white': '#99ccff',
  'light-catppuccin-latte': '#cdbbfb',
  'light-flexoki': '#c9c7c2',
  'light-everforest': '#c1d08d',
  'light-rose-pine-dawn': '#d3bcee',
  'light-cupcake': '#debbd7',
  'light-sakura': '#dab7f3',
  'light-silk': '#7bd9df',
  'dark-black': '#99ccff',
  'dark-flexoki': '#c7c7cc',
  'dark-rose-pine': '#d5b9f7',
  'dark-catppuccin-mocha': '#d5b9f7',
  'dark-nightfox': '#a3caff',
  'dark-dracula': '#d2baf9',
  'dark-nord': '#99d2e2',
  'dark-everforest': '#b9d292',
};
