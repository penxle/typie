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
  manifest: Uint8Array;
};

export const isUnsupportedFontFormat = (err: unknown): boolean => err instanceof Error && err.message.includes('unsplittable font');

export const isNonEmptyHead = (head: { ContentLength?: number } | null): boolean => head !== null && (head.ContentLength ?? 0) > 0;

function findBestStrategy(
  fontName: string,
  fontCps: Set<number>,
  strategies: Record<string, number[][]>,
): { name: string; groups: number[][] } | null {
  const base = fontName.split('-')[0];

  for (const [locale, strategyName] of LOCALE_TO_STRATEGY) {
    if (!(base.endsWith(locale) && Object.hasOwn(strategies, strategyName))) {
      continue;
    }

    const groups = strategies[strategyName];
    const strategyCps = new Set(groups.flat());
    let overlap = 0;
    for (const cp of fontCps) {
      if (strategyCps.has(cp)) overlap++;
    }
    if (overlap >= SLICING_MIN_OVERLAP) return { name: strategyName, groups };
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
    const unionSize = fontCps.union(strategyCps).size;
    const score = overlap / unionSize;
    if (score > bestScore) {
      bestScore = score;
      bestName = name;
      bestGroups = groups;
    }
  }

  return bestName && bestGroups && bestScore >= SLICING_MIN_JACCARD ? { name: bestName, groups: bestGroups } : null;
}

const MAX_CHUNKS = 255;

function sliceSorted(codepoints: number[], size: number): number[][] {
  const out: number[][] = [];
  for (let i = 0; i < codepoints.length; i += size) {
    out.push(codepoints.slice(i, i + size));
  }
  return out;
}

export function chunkCodepoints(
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
    if (chunks.length <= MAX_CHUNKS && remaining.length === 0) {
      return { chunks, strategy: matched.name };
    }
    if (chunks.length < MAX_CHUNKS) {
      const budget = MAX_CHUNKS - chunks.length;
      const size = Math.max(CHUNK_SIZE, Math.ceil(remaining.length / budget));
      chunks.push(...sliceSorted(remaining, size));
      if (chunks.length > MAX_CHUNKS) {
        throw new Error(`chunk budget exceeded: ${chunks.length}`);
      }
      return { chunks, strategy: matched.name };
    }
    // strategy 청크만으로 예산 초과(그리고 remaining 존재) — 전체 순차 분할로 fallback
  }

  const sorted = codepoints.toSorted((a, b) => a - b);
  const size = Math.max(CHUNK_SIZE, Math.ceil(sorted.length / MAX_CHUNKS));
  const chunks = sliceSorted(sorted, size);
  if (chunks.length > MAX_CHUNKS) {
    throw new Error(`chunk budget exceeded: ${chunks.length}`);
  }
  return { chunks, strategy: null };
}

export async function processFont(name: string, ttfData: Uint8Array): Promise<ProcessedFont> {
  const [codepoints, strategies] = await Promise.all([wasm.get_font_codepoints(ttfData), loadStrategies()]);
  const { chunks: chunkCps, strategy } = chunkCodepoints(name, [...codepoints], strategies);
  const output = await wasm.build_font(ttfData, { chunks: chunkCps });

  return {
    hash: output.hash,
    strategy,
    coverages: output.coverage,
    base: output.base,
    chunks: output.chunks,
    manifest: output.manifest,
  };
}

export async function outlineTextToSvg(fontData: Uint8Array, text: string): Promise<string> {
  const raw = await legacyWasm.outlineTextToSvg(fontData, text);
  const { data } = optimize(raw, { multipass: true });
  return data;
}
