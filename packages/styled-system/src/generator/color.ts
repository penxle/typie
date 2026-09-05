import { APCAcontrast, sRGBtoY } from 'apca-w3';
import { converter, formatHex, formatHex8, parse, wcagContrast } from 'culori';

const toOklch = converter('oklch');
const toRgb = converter('rgb');

const parseOrThrow = (value: string) => {
  const color = parse(value);
  if (!color) throw new Error(`unparseable color ${value}`);
  return color;
};

export const normalizeHex = (value: string): string => {
  const color = parseOrThrow(value);
  return color.alpha !== undefined && color.alpha < 1 ? formatHex8(color) : formatHex(color);
};

export const oklchComment = (hex: string): string => {
  const color = toOklch(parseOrThrow(hex));
  const alpha = color.alpha === undefined || color.alpha >= 1 ? '' : ` / ${Math.round(color.alpha * 100)}%`;
  return `oklch(${color.l.toFixed(3)} ${color.c.toFixed(3)} ${(color.h ?? 0).toFixed(0)}${alpha})`;
};

export const withAlpha = (hex: string, alpha: number): string => formatHex8({ ...toRgb(parseOrThrow(hex)), alpha });

export const compositeOver = (fg: string, bg: string): string => {
  const front = toRgb(parseOrThrow(fg));
  const back = toRgb(parseOrThrow(bg));
  const alpha = front.alpha ?? 1;
  return formatHex({
    mode: 'rgb',
    r: front.r * alpha + back.r * (1 - alpha),
    g: front.g * alpha + back.g * (1 - alpha),
    b: front.b * alpha + back.b * (1 - alpha),
  });
};

export const contrastWcag = (fg: string, bg: string): number => wcagContrast(parseOrThrow(fg), parseOrThrow(bg));

const channels = (hex: string): number[] => {
  const color = toRgb(parseOrThrow(hex));
  return [color.r * 255, color.g * 255, color.b * 255];
};

export const contrastApca = (fg: string, bg: string): number =>
  Math.abs(Number(APCAcontrast(sRGBtoY(channels(fg)), sRGBtoY(channels(bg)))));
