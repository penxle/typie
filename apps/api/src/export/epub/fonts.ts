import { mkdir, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { decompressZstd } from '@/utils/compression';
import type { DocumentFontFamily } from '@/utils/document';
import type { NodeEntry, TextSegment } from './types';

export type FontFile = {
  familyName: string;
  weight: number;
  filename: string;
  bytes: Uint8Array;
};

async function compressWoff2(ttf: Uint8Array): Promise<Uint8Array> {
  const dir = path.join(tmpdir(), `woff2-${crypto.randomUUID()}`);
  const ttfPath = path.join(dir, 'font.ttf');
  const woff2Path = path.join(dir, 'font.woff2');

  try {
    await mkdir(dir);
    await Bun.write(ttfPath, ttf);

    const proc = Bun.spawn(['woff2_compress', ttfPath], { stdout: 'ignore', stderr: 'ignore' });
    const exitCode = await proc.exited;
    if (exitCode !== 0) {
      throw new Error(`woff2_compress exited with ${exitCode}`);
    }

    return new Uint8Array(await Bun.file(woff2Path).arrayBuffer());
  } finally {
    await rm(dir, { recursive: true });
  }
}

export function collectUsedFonts(nodes: Record<string, NodeEntry>, defaultFontFamily: string): Set<string> {
  const used = new Set<string>([`${defaultFontFamily}:400`]);

  for (const entry of Object.values(nodes)) {
    if (entry.type !== 'text') continue;

    const segments = (entry.text as TextSegment[]) ?? [];
    for (const seg of segments) {
      let family = defaultFontFamily;
      let weight = 400;

      for (const style of seg.styles ?? []) {
        if (style.type === 'font_family') family = style.family;
        else if (style.type === 'font_weight') weight = style.weight;
        else if (style.type === 'bold') weight = 700;
      }

      used.add(`${family}:${weight}`);
    }
  }

  return used;
}

export async function loadFontFiles(usedFonts: Set<string>, fontFamilies: DocumentFontFamily[]): Promise<Map<string, FontFile>> {
  const result = new Map<string, FontFile>();

  const tasks: Promise<void>[] = [];

  for (const key of usedFonts) {
    const [familyName, weightStr] = key.split(':');
    const weight = Number(weightStr);

    const family = fontFamilies.find((f) => f.familyName === familyName);
    if (!family) continue;

    // see: Rust nearest_weight()
    const font =
      family.fonts.find((f) => f.weight === weight) ??
      family.fonts.reduce<(typeof family.fonts)[number] | null>((prev, curr) => {
        if (!prev) return curr;
        const prevDiff = Math.abs(prev.weight - weight);
        const currDiff = Math.abs(curr.weight - weight);
        if (currDiff < prevDiff) return curr;
        if (currDiff === prevDiff && curr.weight > prev.weight) return curr;
        return prev;
      }, null);
    if (!font) continue;

    tasks.push(
      (async () => {
        try {
          const resp = await fetch(`${font.url}/original.bin`);
          if (!resp.ok) return;

          const compressed = new Uint8Array(await resp.arrayBuffer());
          const ttf = await decompressZstd(compressed);
          const woff2 = await compressWoff2(ttf);

          result.set(key, {
            familyName,
            weight,
            filename: `${familyName}-${weight}.woff2`,
            bytes: woff2,
          });
        } catch {
          // Skip fonts that fail to load
        }
      })(),
    );
  }

  await Promise.all(tasks);
  return result;
}
