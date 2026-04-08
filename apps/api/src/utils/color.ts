import { TinyColor } from '@ctrl/tinycolor';

export function normalizeHexColor(color?: string | null): string | null {
  const normalizedColor = color?.trim();
  if (!normalizedColor) {
    return null;
  }

  const parsedColor = new TinyColor(normalizedColor);
  if (!parsedColor.isValid) {
    return null;
  }

  return parsedColor.toHexString().toUpperCase();
}

export function applySvgRootColor(svg: string, color?: string | null): string {
  if (!color) {
    return svg;
  }

  return svg.replace(/^<svg\b([^>]*)>/, (_match, attrs: string) => {
    if (/\scolor=/.test(attrs)) {
      return `<svg${attrs.replace(/\scolor="[^"]*"/, ` color="${color}"`)}>`;
    }

    return `<svg color="${color}"${attrs}>`;
  });
}
