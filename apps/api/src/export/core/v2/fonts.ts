import { resolveRunStyle } from './style.ts';
import type { PlainDoc, PlainNodeEntry } from '@typie/editor-ffi/server';

export function collectUsedFontsV2(plain: PlainDoc, defaults: { fontFamily: string; fontSizePt100: number }): Set<string> {
  const used = new Set<string>([`${defaults.fontFamily}:400`]);
  const walk = (entry: PlainNodeEntry) => {
    if (entry.node.type === 'text') {
      const s = resolveRunStyle(entry.modifiers, defaults);
      used.add(`${s.fontFamily}:${s.fontWeight}`);
    }
    for (const child of entry.children) walk(child);
  };
  walk(plain.root);
  return used;
}
