import type { Editor, EditorHost } from '@typie/editor-ffi/server';
import type { EditorFontFamily } from './font-families.ts';

const REQUIRED_LOAD_ATTEMPTS = 3;
const LOAD_RETRY_BASE_MS = 200;

type FontData = { type: 'base' } | { type: 'chunk'; id: number };

const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));

const FETCH_CACHE_MAX = 512;
const fetchCache = new Map<string, Promise<Uint8Array>>();
function getOrFetch(url: string): Promise<Uint8Array> {
  let p = fetchCache.get(url);
  if (!p) {
    if (fetchCache.size >= FETCH_CACHE_MAX) {
      const oldest = fetchCache.keys().next().value;
      if (oldest !== undefined) fetchCache.delete(oldest);
    }
    p = (async () => {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`Failed to fetch ${url}: ${res.status}`);
      return new Uint8Array(await res.arrayBuffer());
    })().catch((err) => {
      fetchCache.delete(url);
      throw err;
    });
    fetchCache.set(url, p);
  }
  return p;
}

export type FontRegistration = {
  baseUrlOf: (family: string, weight: number) => string | undefined;
};

export function registerFonts(host: EditorHost, families: readonly EditorFontFamily[]): FontRegistration {
  const baseUrls = new Map<string, string>();
  for (const fam of families) {
    for (const w of fam.weights) {
      baseUrls.set(`${fam.name}:${w.value}`, w.baseUrl);
    }
  }

  host.set_fonts(
    families.map((fam) => ({
      name: fam.name,
      source: fam.source,
      weights: fam.weights.map((w) => ({ value: w.value, hash: w.hash, chunks: w.chunks })),
    })),
  );

  return { baseUrlOf: (family, weight) => baseUrls.get(`${family}:${weight}`) };
}

async function loadOne(host: EditorHost, editor: Editor, family: string, weight: number, fd: FontData, baseUrl: string): Promise<void> {
  const url = fd.type === 'base' ? `${baseUrl}/base` : `${baseUrl}/chunks/${fd.id}`;
  let lastErr: unknown;
  for (let attempt = 1; attempt <= REQUIRED_LOAD_ATTEMPTS; attempt++) {
    try {
      const data = await getOrFetch(url);
      if (fd.type === 'base') {
        host.add_font_base(family, weight, data);
        editor.enqueue({ type: 'system', event: { type: 'font_base_loaded', family, weight } });
      } else {
        host.add_font_chunk(family, weight, fd.id, data);
        editor.enqueue({ type: 'system', event: { type: 'font_chunk_loaded', family, weight, chunk_id: fd.id } });
      }
      return;
    } catch (err) {
      lastErr = err;
      if (attempt < REQUIRED_LOAD_ATTEMPTS) await sleep(LOAD_RETRY_BASE_MS * 2 ** (attempt - 1));
    }
  }
  throw lastErr;
}

export async function handleFontDataMissing(
  host: EditorHost,
  editor: Editor,
  reg: FontRegistration,
  ev: { family: string; weight: number; required: FontData[]; prefetch: FontData[] },
): Promise<void> {
  const baseUrl = reg.baseUrlOf(ev.family, ev.weight);
  if (!baseUrl) {
    console.warn(`[pdf-v2] no font path registered for ${ev.family}:${ev.weight}`);
    return;
  }
  const bases = ev.required.filter((fd) => fd.type === 'base');
  const chunks = ev.required.filter((fd) => fd.type === 'chunk');
  await Promise.allSettled(bases.map((fd) => loadOne(host, editor, ev.family, ev.weight, fd, baseUrl)));
  await Promise.allSettled(chunks.map((fd) => loadOne(host, editor, ev.family, ev.weight, fd, baseUrl)));
}
