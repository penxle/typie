import { and, eq } from 'drizzle-orm';
import { db, first, firstOrThrow, FontFamilies, Fonts, TableCode, validateDbId } from '@/db';
import { FontFamilyState, FontState } from '@/enums';
import { builder } from '../builder';
import { Blob, Font, FontFamily, isTypeOf, User } from '../objects';

Font.implement({
  isTypeOf: isTypeOf(TableCode.FONTS),
  interfaces: [Blob],
  fields: (t) => ({
    name: t.exposeString('name'),
    fullName: t.exposeString('fullName', { nullable: true }),
    weight: t.exposeInt('weight'),

    family: t.expose('familyId', { type: FontFamily }),

    url: t.string({ resolve: (font) => `https://typie.net/fonts/${font.path}` }),
  }),
});

FontFamily.implement({
  isTypeOf: isTypeOf(TableCode.FONT_FAMILIES),
  fields: (t) => ({
    id: t.exposeID('id'),
    name: t.exposeString('name'),

    fonts: t.field({
      type: [Font],
      resolve: async (self) => {
        return await db.select().from(Fonts).where(eq(Fonts.familyId, self.id));
      },
    }),
  }),
});

builder.mutationFields((t) => ({
  archiveFont: t.withAuth({ session: true }).fieldWithInput({
    type: User,
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

      return ctx.session.userId;
    },
  }),
}));
