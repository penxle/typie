import type { EffectiveTheme } from '@typie/ui/context';

export type ThemeColors = {
  background: number;
  text: number;
  colors: Map<string, number>;
};

export const LIGHT_THEME: ThemeColors = {
  background: 0xff_ff_ff_ff,
  text: 0x00_00_00_ff,
  colors: new Map([
    ['inverted', 0xff_ff_ff_ff],
    ['text.subtle', 0x66_66_66_ff],
    ['red', 0xff_00_00_ff],
    ['green', 0x00_ff_00_ff],
    ['blue', 0x00_00_ff_ff],
    ['highlight.yellow', 0xff_f1_76_cc],
    ['highlight.green', 0xa5_d6_a7_cc],
    ['highlight.blue', 0x90_ca_f9_cc],
    ['highlight.pink', 0xf4_8f_b1_cc],
    ['highlight.orange', 0xff_cc_80_cc],
  ]),
};

export const DARK_THEME: ThemeColors = {
  background: 0x1e_1e_1e_ff,
  text: 0xff_ff_ff_ff,
  colors: new Map([
    ['inverted', 0x1e_1e_1e_ff],
    ['text.subtle', 0x99_99_99_ff],
    ['red', 0xbb_00_00_ff],
    ['green', 0x00_bb_00_ff],
    ['blue', 0x00_00_bb_ff],
    ['highlight.yellow', 0xff_f1_76_aa],
    ['highlight.green', 0xa5_d6_a7_aa],
    ['highlight.blue', 0x90_ca_f9_aa],
    ['highlight.pink', 0xf4_8f_b1_aa],
    ['highlight.orange', 0xff_cc_80_aa],
  ]),
};

export const getEditorTheme = (effective: EffectiveTheme): ThemeColors => {
  return effective === 'dark' ? DARK_THEME : LIGHT_THEME;
};

export const formatColor = (color: number) => '#' + color.toString(16).padStart(8, '0').slice(0, 6);
