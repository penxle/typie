import { wasm } from '#/utils/wasm-ffi.ts';
import type { Editor, EditorHost } from '@typie/editor-ffi/server';
import type { EditorFontFamily } from './font-families.ts';

const REQUIRED_LOAD_ATTEMPTS = 3;
const LOAD_RETRY_BASE_MS = 200;

type FontData = { type: 'manifest' } | { type: 'base' } | { type: 'chunk'; id: number };

class FontResourceDeliveryError extends Error {}

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
  failedManifests: ReadonlySet<string>;
};

export function manifestEscalationKey(
  ev: { family: string; weight: number; required: FontData[] },
  failedManifests: ReadonlySet<string>,
): string | null {
  const requiresManifest = ev.required.some((fd) => fd.type === 'manifest');
  if (!requiresManifest) return null;
  const key = `${ev.family}:${ev.weight}`;
  return failedManifests.has(key) ? key : null;
}

export async function registerFonts(host: EditorHost, families: readonly EditorFontFamily[]): Promise<FontRegistration> {
  const baseUrls = new Map<string, string>();
  for (const fam of families) {
    for (const w of fam.weights) {
      baseUrls.set(`${fam.name}:${w.value}`, w.baseUrl);
    }
  }

  host
    .set_fonts(
      families.map((fam) => ({
        name: fam.name,
        source: fam.source,
        weights: fam.weights.map((w) => ({ value: w.value, hash: w.hash })),
      })),
    )
    ?.free();

  const failedManifests = new Set<string>();
  for (const fam of families) {
    for (const w of fam.weights) {
      try {
        const manifest = await wasm.build_font_manifest({ chunks: w.chunks });
        host.add_font_manifest(fam.name, w.value, manifest)?.free();
      } catch (err) {
        failedManifests.add(`${fam.name}:${w.value}`);
        console.warn(`[pdf-v2] manifest registration failed for ${fam.name}:${w.value}`, err);
      }
    }
  }

  return { baseUrlOf: (family, weight) => baseUrls.get(`${family}:${weight}`), failedManifests };
}

async function loadOne(
  host: EditorHost,
  editor: Editor,
  family: string,
  weight: number,
  fd: Exclude<FontData, { type: 'manifest' }>,
  baseUrl: string,
): Promise<void> {
  const url = fd.type === 'base' ? `${baseUrl}/base` : `${baseUrl}/chunks/${fd.id}`;
  const resource = fd.type === 'base' ? 'base' : `chunk ${fd.id}`;
  let lastErr: unknown;
  for (let attempt = 1; attempt <= REQUIRED_LOAD_ATTEMPTS; attempt++) {
    try {
      const data = await getOrFetch(url);
      let update;
      try {
        update = fd.type === 'base' ? host.add_font_base(family, weight, data) : host.add_font_chunk(family, weight, fd.id, data);
      } catch (err) {
        throw new FontResourceDeliveryError(`[pdf-v2] failed to apply font resource ${family}:${weight} ${resource}`, {
          cause: err,
        });
      }
      if (!update) {
        throw new FontResourceDeliveryError(
          `[pdf-v2] requested font resource produced no resource update: ${family}:${weight} ${resource}`,
        );
      }
      try {
        editor.receive_resource_update(update);
      } catch (err) {
        throw new FontResourceDeliveryError(`[pdf-v2] failed to deliver font resource ${family}:${weight} ${resource}`, { cause: err });
      } finally {
        update.free();
      }
      return;
    } catch (err) {
      if (err instanceof FontResourceDeliveryError) throw err;
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
  const bases = ev.required.filter((fd): fd is Extract<FontData, { type: 'base' }> => fd.type === 'base');
  const chunks = ev.required.filter((fd): fd is Extract<FontData, { type: 'chunk' }> => fd.type === 'chunk');
  for (const entries of [bases, chunks]) {
    const results = await Promise.allSettled(entries.map((fd) => loadOne(host, editor, ev.family, ev.weight, fd, baseUrl)));
    let deliveryError: FontResourceDeliveryError | undefined;
    for (const [index, result] of results.entries()) {
      if (result.status === 'fulfilled') continue;
      const fd = entries[index];
      const resource = fd?.type === 'chunk' ? `chunk ${fd.id}` : 'base';
      console.warn(`[pdf-v2] failed to load required font resource ${ev.family}:${ev.weight} ${resource}`, result.reason);
      if (result.reason instanceof FontResourceDeliveryError) deliveryError ??= result.reason;
    }
    if (deliveryError) throw deliveryError;
  }
}
