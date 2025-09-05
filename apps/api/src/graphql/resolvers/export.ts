import { and, eq, inArray } from 'drizzle-orm';
import { db, Entities, firstOrThrowWith, FontFamilies, Fonts, PostContents, Posts, TableCode, validateDbId } from '@/db';
import { EntityVisibility, ExportLayoutMode, FontState, PostLayoutMode } from '@/enums';
import { NotFoundError } from '@/errors';
import { generatePostDocx } from '@/export/docx/docx';
import { extractFontIds, FontMapper } from '@/export/docx/utils/font-mapping';
import { generatePostPDF } from '@/export/pdf';
import { builder } from '../builder';

/**
 * * Types
 */

const ExportPostAsPdfResult = builder.objectRef<{
  data: Uint8Array;
  filename: string;
}>('ExportPostAsPdfResult');

ExportPostAsPdfResult.implement({
  fields: (t) => ({
    data: t.expose('data', { type: 'Binary' }),
    filename: t.exposeString('filename'),
  }),
});

const ExportPostAsDocxResult = builder.objectRef<{
  data: Uint8Array;
  filename: string;
}>('ExportPostAsDocxResult');

ExportPostAsDocxResult.implement({
  fields: (t) => ({
    data: t.expose('data', { type: 'Binary' }),
    filename: t.exposeString('filename'),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  exportPostAsPdf: t.withAuth({ session: true }).fieldWithInput({
    type: ExportPostAsPdfResult,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
      layoutMode: t.input.field({ type: ExportLayoutMode }),
      width: t.input.float(),
      height: t.input.float(),
      marginTop: t.input.float(),
      marginBottom: t.input.float(),
      marginLeft: t.input.float(),
      marginRight: t.input.float(),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db.select().from(Entities).where(eq(Entities.id, input.entityId)).then(firstOrThrowWith(new NotFoundError()));

      if (entity.visibility === EntityVisibility.PRIVATE && entity.userId !== ctx.session.userId) {
        throw new NotFoundError();
      }

      if (entity.type !== 'POST') {
        throw new NotFoundError();
      }

      const accessToken = ctx.c.req.header('Authorization')?.replace('Bearer ', '');

      const [post, pdfBuffer] = await Promise.all([
        db.select().from(Posts).where(eq(Posts.entityId, entity.id)).then(firstOrThrowWith(new NotFoundError())),
        generatePostPDF({
          entitySlug: entity.slug,
          accessToken,
          layoutMode: input.layoutMode,
          pageLayout: {
            width: input.width,
            height: input.height,
            marginTop: input.marginTop,
            marginBottom: input.marginBottom,
            marginLeft: input.marginLeft,
            marginRight: input.marginRight,
          },
        }),
      ]);

      return {
        data: pdfBuffer,
        filename: `${post.title || '(내용 없음)'}${post.subtitle ? ` - ${post.subtitle}` : ''}.pdf`,
      };
    },
  }),
  exportPostAsDocx: t.withAuth({ session: true }).fieldWithInput({
    type: ExportPostAsDocxResult,
    input: {
      entityId: t.input.id({ validate: validateDbId(TableCode.ENTITIES) }),
    },
    resolve: async (_, { input }, ctx) => {
      const entity = await db.select().from(Entities).where(eq(Entities.id, input.entityId)).then(firstOrThrowWith(new NotFoundError()));

      if (entity.visibility === EntityVisibility.PRIVATE && entity.userId !== ctx.session.userId) {
        throw new NotFoundError();
      }

      if (entity.type !== 'POST') {
        throw new NotFoundError();
      }

      const post = await db.select().from(Posts).where(eq(Posts.entityId, entity.id)).then(firstOrThrowWith(new NotFoundError()));
      const postContent = await db
        .select()
        .from(PostContents)
        .where(eq(PostContents.postId, post.id))
        .then(firstOrThrowWith(new NotFoundError()));

      const fontIds = extractFontIds(postContent.body);

      const fontMapper = new FontMapper();

      if (fontIds.size > 0) {
        const fontFamilyIds = [...fontIds].filter((id) => id.startsWith('FNTF'));
        const directFontIds = [...fontIds].filter((id) => id.startsWith('FNTS'));

        if (fontFamilyIds.length > 0) {
          const fontFamilies = await db
            .select({
              id: FontFamilies.id,
              name: FontFamilies.name,
            })
            .from(FontFamilies)
            .where(inArray(FontFamilies.id, fontFamilyIds));

          const familyNameMap = new Map<string, string>();
          for (const family of fontFamilies) {
            familyNameMap.set(family.id, family.name);
            fontMapper.addCustomFont({
              id: family.id,
              fullName: family.name,
              familyName: family.name,
            });
          }

          const fontsInFamilies = await db
            .select({
              id: Fonts.id,
              familyId: Fonts.familyId,
              familyName: Fonts.familyName,
              fullName: Fonts.fullName,
              postScriptName: Fonts.postScriptName,
              weight: Fonts.weight,
            })
            .from(Fonts)
            .where(and(inArray(Fonts.familyId, fontFamilyIds), eq(Fonts.state, FontState.ACTIVE)));

          for (const font of fontsInFamilies) {
            const familyName = familyNameMap.get(font.familyId) || font.familyName;
            fontMapper.addCustomFont({
              id: font.familyId,
              fullName: font.fullName,
              familyName,
              postScriptName: font.postScriptName,
              weight: font.weight,
            });
          }
        }

        if (directFontIds.length > 0) {
          const customFonts = await db
            .select({
              id: Fonts.id,
              familyName: Fonts.familyName,
              fullName: Fonts.fullName,
              postScriptName: Fonts.postScriptName,
              weight: Fonts.weight,
            })
            .from(Fonts)
            .where(and(inArray(Fonts.id, directFontIds), eq(Fonts.state, FontState.ACTIVE)));

          for (const font of customFonts) {
            fontMapper.addCustomFont({
              id: font.id,
              familyName: font.familyName,
              fullName: font.fullName,
              postScriptName: font.postScriptName,
              weight: font.weight,
            });
          }
        }
      }

      const { layoutMode, pageLayout } = postContent;

      const docxBuffer = await generatePostDocx({
        title: post.title,
        subtitle: post.subtitle,
        content: postContent.body,
        text: postContent.text,
        fontMapper,
        layoutMode,
        pageLayout:
          layoutMode === PostLayoutMode.PAGE && pageLayout
            ? {
                width: pageLayout.width,
                height: pageLayout.height,
                marginTop: pageLayout.marginTop,
                marginBottom: pageLayout.marginBottom,
                marginLeft: pageLayout.marginLeft,
                marginRight: pageLayout.marginRight,
              }
            : undefined,
      });

      return {
        data: docxBuffer,
        filename: `${post.title || '(내용 없음)'}${post.subtitle ? ` - ${post.subtitle}` : ''}.docx`,
      };
    },
  }),
}));
