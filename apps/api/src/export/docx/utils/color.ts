import { values as tiptapValues } from '@typie/ui/tiptap/values-base';

const textColorToHex: Record<string, string> = {};
for (const color of tiptapValues.textColor) {
  textColorToHex[color.value] = color.hex.replace('#', '');
}

const textBackgroundColorToHex: Record<string, string> = {};
for (const bgColor of tiptapValues.textBackgroundColor) {
  if (bgColor.value !== 'none' && bgColor.hex) {
    textBackgroundColorToHex[bgColor.value] = bgColor.hex.replace('#', '');
  }
}

export function normalizeColor(color: string | undefined, isBackground = false): string | undefined {
  if (!color || color === 'none') {
    return undefined;
  }

  if (isBackground && textBackgroundColorToHex[color]) {
    return textBackgroundColorToHex[color];
  }

  if (!isBackground && textColorToHex[color]) {
    return textColorToHex[color];
  }

  let hex = color.trim().toLowerCase();

  if (hex.startsWith('#')) {
    hex = hex.slice(1);
  }

  if (hex.length === 3) {
    hex = [...hex].map((char) => char + char).join('');
  }

  if (hex.length === 6 && /^[0-9a-f]{6}$/i.test(hex)) {
    return hex.toUpperCase();
  }

  return undefined;
}
