/* tslint:disable */
/* eslint-disable */
export class GlyphBitmap {
  private constructor();
  free(): void;
  readonly width: number;
  readonly height: number;
  readonly top: number;
  readonly left: number;
  readonly buffer: Uint8Array;
}
export class GlyphMetrics {
  private constructor();
  free(): void;
  readonly advance_width: number;
  readonly advance_height: number;
  readonly lsb: number;
  readonly tsb: number;
  readonly vertical_origin: number;
}
export class Glyphr {
  free(): void;
  constructor();
  load_font(font_data: Uint8Array): void;
  get_metrics(char_code: number): GlyphMetrics;
  render_glyph(char_code: number): GlyphBitmap;
}

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly __wbg_glyphbitmap_free: (a: number, b: number) => void;
  readonly glyphbitmap_width: (a: number) => number;
  readonly glyphbitmap_height: (a: number) => number;
  readonly glyphbitmap_top: (a: number) => number;
  readonly glyphbitmap_left: (a: number) => number;
  readonly glyphbitmap_buffer: (a: number) => any;
  readonly __wbg_glyphmetrics_free: (a: number, b: number) => void;
  readonly glyphmetrics_advance_width: (a: number) => number;
  readonly glyphmetrics_advance_height: (a: number) => number;
  readonly glyphmetrics_lsb: (a: number) => number;
  readonly glyphmetrics_tsb: (a: number) => number;
  readonly glyphmetrics_vertical_origin: (a: number) => number;
  readonly __wbg_glyphr_free: (a: number, b: number) => void;
  readonly glyphr_new: () => number;
  readonly glyphr_load_font: (a: number, b: number, c: number) => [number, number];
  readonly glyphr_get_metrics: (a: number, b: number) => [number, number, number];
  readonly glyphr_render_glyph: (a: number, b: number) => [number, number, number];
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_export_3: WebAssembly.Table;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
