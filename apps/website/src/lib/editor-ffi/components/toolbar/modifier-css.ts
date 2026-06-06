import type { Modifier } from '@typie/editor-ffi/browser';

const PT_TO_PX = 96 / 72;

type Options = {
  textColorMap: ReadonlyMap<string, string>;
  bgColorMap: ReadonlyMap<string, string | null>;
  maxFontSize?: number;
};

export const modifiersToCss = (modifiers: readonly Modifier[], { textColorMap, bgColorMap, maxFontSize }: Options): string => {
  const parts: string[] = [];
  const decorations: string[] = [];

  for (const mod of modifiers) {
    switch (mod.type) {
      case 'bold': {
        parts.push('font-weight: 700');
        break;
      }
      case 'italic': {
        parts.push('font-style: italic');
        break;
      }
      case 'underline': {
        decorations.push('underline');
        break;
      }
      case 'strikethrough': {
        decorations.push('line-through');
        break;
      }
      case 'font_size': {
        const px = (mod.value / 100) * PT_TO_PX;
        parts.push(`font-size: ${maxFontSize === undefined ? px : Math.min(px, maxFontSize)}px`);
        break;
      }
      case 'font_weight': {
        parts.push(`font-weight: ${mod.value}`);
        break;
      }
      case 'font_family': {
        parts.push(`font-family: ${mod.value}`);
        break;
      }
      case 'text_color': {
        const color = textColorMap.get(mod.value) ?? mod.value;
        parts.push(`color: ${color}`);
        break;
      }
      case 'background_color': {
        const color = bgColorMap.get(mod.value);
        if (color) parts.push(`background-color: ${color}`);
        break;
      }
    }
  }

  if (decorations.length > 0) parts.push(`text-decoration: ${decorations.join(' ')}`);
  return parts.join('; ');
};
