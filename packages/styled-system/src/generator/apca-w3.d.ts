declare module 'apca-w3' {
  export function sRGBtoY(rgb: readonly number[]): number;
  export function APCAcontrast(textY: number, backgroundY: number, places?: number): number;
}
