const MM_TO_INCH = 1 / 25.4; // 1 inch = 25.4mm
const INCH_TO_PT = 72; // 1 inch = 72pt
const PT_TO_TWIPS = 20; // 1pt = 20 twips
const MM_TO_TWIPS = MM_TO_INCH * INCH_TO_PT * PT_TO_TWIPS; // 1mm = 56.6929 twips
const PX_TO_PT = 0.75; // 1px = 0.75pt (96 DPI)
const REM_TO_PX = 16; // 1rem = 16px
const EM_TO_PX = 16; // 1em = 16px
const INCH_TO_PIXELS = 96; // 1 inch = 96 pixels (96 DPI)

export const pxToPt = (px: number) => px * PX_TO_PT;
export const pxToHalfPt = (px: number) => px * PX_TO_PT * 2;
export const ptToTwips = (pt: number) => pt * PT_TO_TWIPS;
export const pxToTwips = (px: number) => px * PX_TO_PT * PT_TO_TWIPS;
export const mmToTwips = (mm: number) => mm * MM_TO_TWIPS;
export const mmToInch = (mm: number) => mm * MM_TO_INCH;
export const remToPx = (rem: number) => rem * REM_TO_PX;
export const emToPx = (em: number) => em * EM_TO_PX;
export const remToPt = (rem: number) => remToPx(rem) * PX_TO_PT;
export const emToPt = (em: number) => emToPx(em) * PX_TO_PT;
export const inchToPx = (inch: number) => inch * INCH_TO_PIXELS;
