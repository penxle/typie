import defaultFontFamilies from '@typie/editor/font/defaults.json' with { type: 'json' };
import { asc, eq, inArray } from 'drizzle-orm';
import { db, FontFamilies, FontNames, Fonts } from '@/db';
import { FontFamilySource, FontFamilyState, FontState } from '@/enums';

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
  }[];
};

const LANG_KO_KR = 0x04_12;
const LANG_EN_US = 0x04_09;
const PLATFORM_WINDOWS = 3;

type NameEntry = { nameId: number; platformId: number; languageId: number; value: string };

const findNameKo = (records: NameEntry[], nameId: number) =>
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_KO_KR)?.value ??
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_EN_US)?.value ??
  records.find((n) => n.nameId === nameId)?.value;

export async function getDocumentFontFamilies(userId: string): Promise<DocumentFontFamily[]> {
  const rows = await db
    .select({
      familyId: FontFamilies.id,
      familyName: FontFamilies.familyName,
      familyState: FontFamilies.state,
      fontId: Fonts.id,
      fontWeight: Fonts.weight,
      fontPath: Fonts.path,
      fontState: Fonts.state,
    })
    .from(FontFamilies)
    .innerJoin(Fonts, eq(Fonts.familyId, FontFamilies.id))
    .where(eq(FontFamilies.userId, userId))
    .orderBy(asc(FontFamilies.familyName), asc(Fonts.weight));

  const defaults: DocumentFontFamily[] = defaultFontFamilies.map((f) => {
    const firstNames = f.fonts[0]?.names ?? [];
    return {
      id: f.id,
      displayName: findNameKo(firstNames, 16) ?? findNameKo(firstNames, 1) ?? f.familyName,
      familyName: f.familyName,
      source: FontFamilySource.DEFAULT,
      state: FontFamilyState.ACTIVE,
      fonts: f.fonts.map((v) => ({
        id: v.id,
        weight: v.weight,
        subfamilyDisplayName: findNameKo(v.names, 17) ?? findNameKo(v.names, 2) ?? null,
        url: `https://cdn.typie.net/editor/fonts/${v.path}`,
        state: FontState.ACTIVE,
      })),
    };
  });

  if (rows.length === 0) {
    return defaults;
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
      const names = namesByFont.get(row.fontId) ?? [];
      last = {
        id: row.familyId,
        displayName: findNameKo(names, 16) ?? findNameKo(names, 1) ?? row.familyName,
        familyName: row.familyName,
        source: FontFamilySource.USER,
        state: row.familyState,
        fonts: [],
      };
      userFamilies.push(last);
    }
    const names = namesByFont.get(row.fontId) ?? [];
    last.fonts.push({
      id: row.fontId,
      weight: row.fontWeight,
      subfamilyDisplayName: findNameKo(names, 17) ?? findNameKo(names, 2) ?? null,
      url: `https://typie.net/fonts/${row.fontPath}`,
      state: row.fontState,
    });
  }

  return [...defaults, ...userFamilies.toSorted((a, b) => a.displayName.localeCompare(b.displayName))];
}
