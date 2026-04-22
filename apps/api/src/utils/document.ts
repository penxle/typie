import fontFamilies from '@typie/assets/fonts.json' with { type: 'json' };
import { FontFamilySource, FontFamilyState, FontState } from '@typie/lib/enums';
import { asc, eq, inArray } from 'drizzle-orm';
import { db, FontFamilies, FontNames, Fonts } from '#/db/index.ts';

export type DocumentFontFamily = {
  id: string;
  displayName: string;
  familyName: string;
  source: FontFamilySource;
  state: FontFamilyState;
  fonts: {
    id: string;
    weight: number;
    subfamilyDisplayName: string | null;
    url: string;
    state: FontState;
    path: string;
    hash: string;
    /** chunk별 flat 페어 `[start0, end0, start1, end1, ...]` (inclusive). */
    chunks: number[][];
  }[];
};

type FontEntry = {
  id: string;
  postScriptName: string;
  weight: number;
  path: string;
  hash: string;
  chunks: number[][];
  names: NameEntry[];
};

type FontFamilyEntry = {
  id: string;
  familyName: string;
  source: 'DEFAULT' | 'FALLBACK';
  fonts: FontEntry[];
};

const allFamilies = fontFamilies as unknown as FontFamilyEntry[];
const defaultFamilies = allFamilies.filter((f) => f.source === 'DEFAULT');
const fallbackFamilies = allFamilies.filter((f) => f.source === 'FALLBACK');

const LANG_KO_KR = 0x04_12;
const LANG_EN_US = 0x04_09;
const PLATFORM_WINDOWS = 3;

type NameEntry = { nameId: number; platformId: number; languageId: number; value: string };

const DISPLAY_NAME_OVERRIDES: Record<string, string> = {
  Pretendard: '프리텐다드',
  KoPubWorldDotum: '코펍월드돋움',
  KoPubWorldBatang: '코펍월드바탕',
};

const findNameKo = (records: NameEntry[], nameId: number) =>
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_KO_KR)?.value ??
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_EN_US)?.value ??
  records.find((n) => n.nameId === nameId)?.value;

function loadDefaultFontFamilies(): DocumentFontFamily[] {
  return defaultFamilies.map((f) => {
    const firstNames = f.fonts[0]?.names ?? [];
    return {
      id: f.id,
      displayName: DISPLAY_NAME_OVERRIDES[f.familyName] ?? findNameKo(firstNames, 16) ?? findNameKo(firstNames, 1) ?? f.familyName,
      familyName: f.familyName,
      source: FontFamilySource.DEFAULT,
      state: FontFamilyState.ACTIVE,
      fonts: f.fonts.map((v) => ({
        id: v.id,
        weight: v.weight,
        subfamilyDisplayName: findNameKo(v.names, 17) ?? findNameKo(v.names, 2) ?? null,
        url: `https://cdn.typie.net/editor/fonts/${v.path}`,
        state: FontState.ACTIVE,
        path: v.path,
        hash: v.hash ?? '',
        chunks: v.chunks ?? [],
      })),
    };
  });
}

async function loadUserFontFamilies(userId: string): Promise<DocumentFontFamily[]> {
  const rows = await db
    .select({
      familyId: FontFamilies.id,
      familyName: FontFamilies.familyName,
      familyState: FontFamilies.state,
      fontId: Fonts.id,
      fontWeight: Fonts.weight,
      fontPath: Fonts.path,
      fontState: Fonts.state,
      fontHash: Fonts.hash,
      fontChunks: Fonts.chunks,
    })
    .from(FontFamilies)
    .innerJoin(Fonts, eq(Fonts.familyId, FontFamilies.id))
    .where(eq(FontFamilies.userId, userId))
    .orderBy(asc(FontFamilies.familyName), asc(Fonts.weight));

  if (rows.length === 0) {
    return [];
  }

  const fontIds = rows.map((r) => r.fontId);
  const names = await db
    .select({
      fontId: FontNames.fontId,
      nameId: FontNames.nameId,
      platformId: FontNames.platformId,
      languageId: FontNames.languageId,
      value: FontNames.value,
    })
    .from(FontNames)
    .where(inArray(FontNames.fontId, fontIds));

  const namesByFont = new Map<string, typeof names>();
  for (const record of names) {
    let arr = namesByFont.get(record.fontId);
    if (!arr) {
      arr = [];
      namesByFont.set(record.fontId, arr);
    }
    arr.push(record);
  }

  const userFamilies: DocumentFontFamily[] = [];
  for (const row of rows) {
    let last = userFamilies.at(-1);
    if (!last || last.id !== row.familyId) {
      const familyNames = namesByFont.get(row.fontId) ?? [];
      last = {
        id: row.familyId,
        displayName: findNameKo(familyNames, 16) ?? findNameKo(familyNames, 1) ?? row.familyName,
        familyName: row.familyName,
        source: FontFamilySource.USER,
        state: row.familyState,
        fonts: [],
      };
      userFamilies.push(last);
    }
    const fontNames = namesByFont.get(row.fontId) ?? [];
    last.fonts.push({
      id: row.fontId,
      weight: row.fontWeight,
      subfamilyDisplayName: findNameKo(fontNames, 17) ?? findNameKo(fontNames, 2) ?? null,
      url: `https://typie.net/fonts/${row.fontPath}`,
      state: row.fontState,
      path: row.fontPath,
      hash: row.fontHash,
      chunks: row.fontChunks as never,
    });
  }

  return userFamilies.toSorted((a, b) => a.displayName.localeCompare(b.displayName));
}

function loadFallbackFontFamilies(): DocumentFontFamily[] {
  return fallbackFamilies.map((f) => ({
    id: f.id,
    displayName: f.familyName,
    familyName: f.familyName,
    source: FontFamilySource.FALLBACK,
    state: FontFamilyState.ACTIVE,
    fonts: f.fonts.map((v) => {
      if (v.hash === undefined || v.chunks === undefined) {
        throw new Error(
          `Fallback font ${f.familyName}/${v.path} is missing hash or chunks. ` +
            `Regenerate assets/fonts.json via \`bun run apps/api/scripts/build-fonts.ts <source-dir>\`.`,
        );
      }
      return {
        id: v.id,
        weight: v.weight,
        subfamilyDisplayName: null,
        url: '',
        state: FontState.ACTIVE,
        path: v.path,
        hash: v.hash,
        chunks: v.chunks,
      };
    }),
  }));
}

export async function getDocumentFontFamilies(
  ownerId: string,
  viewerId: string | null,
  sources: FontFamilySource[] = [FontFamilySource.DEFAULT, FontFamilySource.USER],
): Promise<DocumentFontFamily[]> {
  const sourceSet = new Set(sources);
  const result: DocumentFontFamily[] = [];

  if (sourceSet.has(FontFamilySource.DEFAULT)) {
    result.push(...loadDefaultFontFamilies());
  }

  if (sourceSet.has(FontFamilySource.USER)) {
    const ownerFonts = await loadUserFontFamilies(ownerId);
    if (viewerId && viewerId !== ownerId) {
      const viewerFonts = await loadUserFontFamilies(viewerId);
      const byName = new Map<string, DocumentFontFamily>();
      for (const f of ownerFonts) byName.set(f.familyName, f);
      for (const f of viewerFonts) byName.set(f.familyName, f);
      result.push(...byName.values());
    } else {
      result.push(...ownerFonts);
    }
  }

  if (sourceSet.has(FontFamilySource.FALLBACK)) {
    result.push(...loadFallbackFontFamilies());
  }

  return result;
}
