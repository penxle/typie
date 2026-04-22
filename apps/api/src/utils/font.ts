import { optimize } from 'svgo';
import { wasm as legacyWasm } from './wasm.ts';
import { wasm } from './wasm-ffi.ts';

const CHUNK_SIZE = 200;
const SLICING_MIN_OVERLAP = 1000;
const SLICING_MIN_JACCARD = 0.5;

const LOCALE_TO_STRATEGY: [string, string][] = [
  ['KR', 'korean'],
  ['JP', 'japanese'],
  ['SC', 'simplified-chinese'],
  ['TC', 'traditional-chinese'],
  ['HK', 'hongkong-chinese'],
];

const SLICING_BASE_URL = 'https://raw.githubusercontent.com/googlefonts/nam-files/main/slices';
const SLICING_FILES: Record<string, string> = {
  korean: 'korean_default.txt',
  japanese: 'japanese_default.txt',
  'simplified-chinese': 'simplified-chinese_default.txt',
  'traditional-chinese': 'traditional-chinese_default.txt',
  'hongkong-chinese': 'hongkong-chinese_default.txt',
};

function parseSlicingFile(text: string): number[][] {
  const groups: number[][] = [];
  let current: number[] | null = null;

  for (const line of text.split('\n')) {
    const stripped = line.trim();
    if (stripped === 'subsets {') {
      current = [];
    } else if (stripped === '}' && current !== null) {
      if (current.length > 0) groups.push(current);
      current = null;
    } else if (current !== null && stripped.startsWith('codepoints:')) {
      const cpStr = stripped.split(':')[1].split('#')[0].trim();
      current.push(Number.parseInt(cpStr, 10));
    }
  }

  return groups;
}

let strategiesCache: Record<string, number[][]> | null = null;

async function loadStrategies(): Promise<Record<string, number[][]>> {
  if (strategiesCache) return strategiesCache;

  const entries = await Promise.all(
    Object.entries(SLICING_FILES).map(async ([lang, filename]) => {
      const resp = await fetch(`${SLICING_BASE_URL}/${filename}`);
      if (!resp.ok) throw new Error(`Failed to fetch slicing data: ${lang}`);
      const text = await resp.text();
      return [lang, parseSlicingFile(text)] as const;
    }),
  );

  strategiesCache = Object.fromEntries(entries);
  return strategiesCache;
}

export type ProcessedFont = {
  hash: string;
  strategy: string | null;
  /** chunk별 flat 페어 `[start0, end0, start1, end1, ...]` (inclusive). */
  coverages: number[][];
  base: Uint8Array;
  chunks: Uint8Array[];
};

function findBestStrategy(
  fontName: string,
  fontCps: Set<number>,
  strategies: Record<string, number[][]>,
): { name: string; groups: number[][] } | null {
  const base = fontName.split('-')[0];

  for (const [locale, strategyName] of LOCALE_TO_STRATEGY) {
    if (base.endsWith(locale) && strategyName in strategies) {
      const groups = strategies[strategyName];
      const strategyCps = new Set(groups.flat());
      let overlap = 0;
      for (const cp of fontCps) {
        if (strategyCps.has(cp)) overlap++;
      }
      if (overlap >= SLICING_MIN_OVERLAP) return { name: strategyName, groups };
    }
  }

  let bestName: string | null = null;
  let bestGroups: number[][] | null = null;
  let bestScore = 0;

  for (const [name, groups] of Object.entries(strategies)) {
    const strategyCps = new Set(groups.flat());
    let overlap = 0;
    for (const cp of fontCps) {
      if (strategyCps.has(cp)) overlap++;
    }
    if (overlap < SLICING_MIN_OVERLAP) continue;
    const unionSize = new Set([...fontCps, ...strategyCps]).size;
    const score = overlap / unionSize;
    if (score > bestScore) {
      bestScore = score;
      bestName = name;
      bestGroups = groups;
    }
  }

  return bestName && bestGroups && bestScore >= SLICING_MIN_JACCARD ? { name: bestName, groups: bestGroups } : null;
}

function chunkCodepoints(
  fontName: string,
  codepoints: number[],
  strategies: Record<string, number[][]>,
): { chunks: number[][]; strategy: string | null } {
  const fontCps = new Set(codepoints);
  const matched = findBestStrategy(fontName, fontCps, strategies);

  if (matched) {
    const covered = new Set<number>();
    const chunks: number[][] = [];

    for (const group of matched.groups) {
      const chunk = group.filter((cp) => fontCps.has(cp));
      if (chunk.length > 0) {
        for (const cp of chunk) covered.add(cp);
        chunks.push(chunk);
      }
    }

    const remaining = codepoints.filter((cp) => !covered.has(cp)).toSorted((a, b) => a - b);
    for (let i = 0; i < remaining.length; i += CHUNK_SIZE) {
      chunks.push(remaining.slice(i, i + CHUNK_SIZE));
    }
    return { chunks, strategy: matched.name };
  }

  const sorted = codepoints.toSorted((a, b) => a - b);
  const chunks: number[][] = [];
  for (let i = 0; i < sorted.length; i += CHUNK_SIZE) {
    chunks.push(sorted.slice(i, i + CHUNK_SIZE));
  }
  return { chunks, strategy: null };
}

export async function processFont(name: string, ttfData: Uint8Array): Promise<ProcessedFont> {
  const [codepoints, strategies] = await Promise.all([wasm.get_font_codepoints(ttfData), loadStrategies()]);
  const { chunks: chunkCps, strategy } = chunkCodepoints(name, codepoints, strategies);
  const output = await wasm.build_font(ttfData, chunkCps);

  return {
    hash: output.hash,
    strategy,
    coverages: output.coverage,
    base: output.base,
    chunks: output.chunks,
  };
}

export async function outlineTextToSvg(fontData: Uint8Array, text: string): Promise<string> {
  const raw = await legacyWasm.outlineTextToSvg(fontData, text);
  const { data } = optimize(raw, { multipass: true });
  return data;
}
