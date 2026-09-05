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

export const VARIANT_SWATCH: Record<ThemeVariant, readonly [string, string, string, string]> = {
  'light-white': ['#ef4444', '#eab308', '#22c55e', '#3b82f6'],
  'light-catppuccin-latte': ['#d20f39', '#df8e1d', '#40a02b', '#1e66f5'],
  'light-flexoki': ['#af3029', '#ad8301', '#66800b', '#205ea6'],
  'light-everforest': ['#f85552', '#dfa000', '#8da101', '#3a94c5'],
  'light-rose-pine-dawn': ['#b4637a', '#ea9d34', '#56949f', '#286983'],
  'light-cupcake': ['#d57e85', '#dcb16c', '#a3b367', '#7297b9'],
  'light-sakura': ['#df2d52', '#c29461', '#2e916d', '#006e93'],
  'light-silk': ['#cf432e', '#cfad25', '#6ca38c', '#39aac9'],
  'dark-black': ['#ef4444', '#eab308', '#22c55e', '#3b82f6'],
  'dark-flexoki': ['#d14d41', '#d0a215', '#879a39', '#4385be'],
  'dark-rose-pine': ['#eb6f92', '#f6c177', '#95b1ac', '#31748f'],
  'dark-catppuccin-mocha': ['#f38ba8', '#f9e2af', '#a6e3a1', '#89b4fa'],
  'dark-nightfox': ['#c94f6d', '#dbc074', '#81b29a', '#719cd6'],
  'dark-dracula': ['#ff5555', '#f1fa8c', '#50fa7b', '#9bc4ff'],
  'dark-nord': ['#bf616a', '#ebcb8b', '#a3be8c', '#81a1c1'],
  'dark-everforest': ['#e67e80', '#dbbc7f', '#a7c080', '#7fbbb3'],
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
