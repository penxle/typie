import { resolveRunStyle } from './style.ts';
import type { PlainDoc } from '@typie/editor-ffi/server';

export function collectUsedFontsV2(plain: PlainDoc, defaults: { fontFamily: string; fontSizePt100: number }): Set<string> {
  const used = new Set<string>([`${defaults.fontFamily}:400`]);
  for (const entry of Object.values(plain.nodes)) {
    if (entry.node.type !== 'text') continue;
    const s = resolveRunStyle(entry.modifiers, defaults);
    used.add(`${s.fontFamily}:${s.fontWeight}`);
  }
  return used;
}
