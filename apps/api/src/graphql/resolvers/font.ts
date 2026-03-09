import { and, asc, eq, inArray } from 'drizzle-orm';
import { db, first, firstOrThrow, FontFamilies, FontNames, Fonts, TableCode, validateDbId } from '@/db';
import { FontFamilyState, FontState } from '@/enums';
import { builder } from '../builder';
import { Blob, Font, FontFamily, isTypeOf } from '../objects';

const LANG_KO_KR = 0x04_12;
const LANG_EN_US = 0x04_09;
const PLATFORM_WINDOWS = 3;

type NameRecord = { nameId: number; platformId: number; languageId: number; value: string };

const findNameEn = (records: NameRecord[], nameId: number) =>
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_EN_US)?.value ??
  records.find((n) => n.nameId === nameId)?.value;

const findNameKo = (records: NameRecord[], nameId: number) =>
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_KO_KR)?.value ??
  records.find((n) => n.nameId === nameId && n.platformId === PLATFORM_WINDOWS && n.languageId === LANG_EN_US)?.value ??
  records.find((n) => n.nameId === nameId)?.value;

Font.implement({
  isTypeOf: isTypeOf(TableCode.FONTS),
  interfaces: [Blob],
  fields: (t) => ({
    fullName: t.string({
      nullable: true,
      resolve: async (font, _, ctx) => {
        const records = await ctx
          .loader({
            name: 'Font.nameRecords',
            many: true,
            load: (ids: string[]) =>
              db
                .select({
                  fontId: FontNames.fontId,
                  nameId: FontNames.nameId,
                  platformId: FontNames.platformId,
                  languageId: FontNames.languageId,
                  value: FontNames.value,
                })
                .from(FontNames)
                .where(inArray(FontNames.fontId, ids)),
            key: (row) => row.fontId,
          })
          .load(font.id);
        return findNameEn(records, 4) ?? null;
      },
    }),

    subfamilyDisplayName: t.string({
      nullable: true,
      resolve: async (font, _, ctx) => {
        const records = await ctx
          .loader({
            name: 'Font.nameRecords',
            many: true,
            load: (ids: string[]) =>
              db
                .select({
                  fontId: FontNames.fontId,
                  nameId: FontNames.nameId,
                  platformId: FontNames.platformId,
                  languageId: FontNames.languageId,
                  value: FontNames.value,
                })
                .from(FontNames)
                .where(inArray(FontNames.fontId, ids)),
            key: (row) => row.fontId,
          })
          .load(font.id);
        return findNameKo(records, 17) ?? findNameKo(records, 2) ?? null;
      },
    }),

    weight: t.exposeInt('weight'),

    family: t.expose('familyId', { type: FontFamily }),

    url: t.string({ resolve: (font) => `https://typie.net/fonts/${font.path}` }),
  }),
});

FontFamily.implement({
  isTypeOf: isTypeOf(TableCode.FONT_FAMILIES),
  fields: (t) => ({
    id: t.exposeID('id'),

    familyName: t.exposeString('familyName'),

    displayName: t.string({
      resolve: async (self, _, ctx) => {
        const records = await ctx
          .loader({
            name: 'FontFamily.nameRecords',
            many: true,
            load: (ids: string[]) =>
              db
                .select({
                  familyId: Fonts.familyId,
                  nameId: FontNames.nameId,
                  platformId: FontNames.platformId,
                  languageId: FontNames.languageId,
                  value: FontNames.value,
                })
                .from(FontNames)
                .innerJoin(Fonts, eq(Fonts.id, FontNames.fontId))
                .where(inArray(Fonts.familyId, ids)),
            key: (row) => row.familyId,
          })
          .load(self.id);
        return findNameKo(records, 16) ?? findNameKo(records, 1) ?? self.familyName;
      },
    }),

    fonts: t.field({
      type: [Font],
      resolve: async (self) => {
        return await db
          .select()
          .from(Fonts)
          .where(and(eq(Fonts.familyId, self.id), eq(Fonts.state, FontState.ACTIVE)))
          .orderBy(asc(Fonts.weight));
      },
    }),
  }),
});

builder.mutationFields((t) => ({
  archiveFontFamily: t.withAuth({ session: true }).fieldWithInput({
    type: FontFamily,
    input: { fontFamilyId: t.input.id({ validate: validateDbId(TableCode.FONT_FAMILIES) }) },
    resolve: async (_, { input }, ctx) => {
      await db
        .select({ id: FontFamilies.id })
        .from(FontFamilies)
        .where(and(eq(FontFamilies.id, input.fontFamilyId), eq(FontFamilies.userId, ctx.session.userId)))
        .then(firstOrThrow);

      await db.update(Fonts).set({ state: FontState.ARCHIVED }).where(eq(Fonts.familyId, input.fontFamilyId));
      await db.update(FontFamilies).set({ state: FontFamilyState.ARCHIVED }).where(eq(FontFamilies.id, input.fontFamilyId));

      return input.fontFamilyId;
    },
  }),

  archiveFont: t.withAuth({ session: true }).fieldWithInput({
    type: Font,
    input: { fontId: t.input.id({ validate: validateDbId(TableCode.FONTS) }) },
    resolve: async (_, { input }, ctx) => {
      const font = await db
        .select({ familyId: Fonts.familyId })
        .from(Fonts)
        .innerJoin(FontFamilies, eq(Fonts.familyId, FontFamilies.id))
        .where(and(eq(Fonts.id, input.fontId), eq(FontFamilies.userId, ctx.session.userId)))
        .then(firstOrThrow);

      await db.update(Fonts).set({ state: FontState.ARCHIVED }).where(eq(Fonts.id, input.fontId));

      const existingFont = await db
        .select({ id: Fonts.id })
        .from(Fonts)
        .where(and(eq(Fonts.familyId, font.familyId), eq(Fonts.state, FontState.ACTIVE)))
        .then(first);

      if (!existingFont) {
        await db.update(FontFamilies).set({ state: FontFamilyState.ARCHIVED }).where(eq(FontFamilies.id, font.familyId));
      }

      return input.fontId;
    },
  }),
}));
