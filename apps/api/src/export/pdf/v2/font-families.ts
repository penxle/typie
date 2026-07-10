import { FontFamilySource } from '@typie/lib/enums';
import { getDocumentFontFamilies } from '#/utils/document.ts';

export type EditorFontFamily = {
  name: string;
  source: 'DEFAULT' | 'USER' | 'FALLBACK';
  weights: { value: number; hash: string; chunks: number[][]; baseUrl: string }[];
};

export async function buildEditorFontFamilies(userId: string): Promise<EditorFontFamily[]> {
  const families = await getDocumentFontFamilies(userId, null, [
    FontFamilySource.DEFAULT,
    FontFamilySource.USER,
    FontFamilySource.FALLBACK,
  ]);

  return families.map((family) => ({
    name: family.familyName,
    source: family.source,
    weights: family.fonts.map((font) => ({
      value: font.weight,
      hash: font.hash,
      chunks: font.chunks,
      baseUrl: `${font.url}/${font.hash}`,
    })),
  }));
}
