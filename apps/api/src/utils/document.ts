import { asc, eq } from 'drizzle-orm';
import { DEFAULT_FONT_FAMILIES } from '@/const';
import { db, FontFamilies, Fonts } from '@/db';
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

export async function getDocumentFontFamilies(userId: string): Promise<DocumentFontFamily[]> {
  const rows = await db
    .select({
      familyId: FontFamilies.id,
      familyName: FontFamilies.familyName,
      familyDisplayName: FontFamilies.displayName,
      familyState: FontFamilies.state,
      fontId: Fonts.id,
      fontWeight: Fonts.weight,
      fontSubfamilyName: Fonts.subfamilyDisplayName,
      fontPath: Fonts.path,
      fontState: Fonts.state,
    })
    .from(FontFamilies)
    .innerJoin(Fonts, eq(Fonts.familyId, FontFamilies.id))
    .where(eq(FontFamilies.userId, userId))
    .orderBy(asc(FontFamilies.familyName), asc(Fonts.weight));

  const userFamilies: DocumentFontFamily[] = [];
  for (const row of rows) {
    let last = userFamilies.at(-1);
    if (!last || last.id !== row.familyId) {
      last = {
        id: row.familyId,
        displayName: row.familyDisplayName,
        familyName: row.familyName,
        source: FontFamilySource.USER,
        state: row.familyState,
        fonts: [],
      };
      userFamilies.push(last);
    }
    last.fonts.push({
      id: row.fontId,
      weight: row.fontWeight,
      subfamilyDisplayName: row.fontSubfamilyName,
      url: `https://typie.net/fonts/${row.fontPath}`,
      state: row.fontState,
    });
  }

  const defaults: DocumentFontFamily[] = DEFAULT_FONT_FAMILIES.map((f) => ({
    id: f.id,
    displayName: f.displayName,
    familyName: f.familyName,
    source: FontFamilySource.DEFAULT,
    state: FontFamilyState.ACTIVE,
    fonts: f.fonts.map((v) => ({
      id: v.id,
      weight: v.weight,
      subfamilyDisplayName: null,
      url: `https://cdn.typie.net/editor/fonts/${v.path}`,
      state: FontState.ACTIVE,
    })),
  }));

  return [...defaults, ...userFamilies];
}
