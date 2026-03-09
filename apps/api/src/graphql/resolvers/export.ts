import { asc, eq } from 'drizzle-orm';
import { DEFAULT_FONT_FAMILIES } from '@/const';
import {
  db,
  DocumentContents,
  Documents,
  Entities,
  firstOrThrow,
  firstOrThrowWith,
  FontFamilies,
  Fonts,
  TableCode,
  Users,
  validateDbId,
} from '@/db';
import { DocumentExportFormat, EntityVisibility } from '@/enums';
import { NotFoundError, TypieError } from '@/errors';
import { generateDocumentDocx, generateDocumentEpub, generateDocumentHwp, generateDocumentPdf } from '@/export/document';
import { getDocumentFontFamilies } from '@/utils/document';
import { builder } from '../builder';

/**
 * * Types
 */

const ExportDocumentPageLayoutInput = builder.inputType('ExportDocumentPageLayoutInput', {
  fields: (t) => ({
    pageWidth: t.int({ required: true }),
    pageHeight: t.int({ required: true }),
    pageMarginTop: t.int({ required: true }),
    pageMarginBottom: t.int({ required: true }),
    pageMarginLeft: t.int({ required: true }),
    pageMarginRight: t.int({ required: true }),
  }),
});

const ExportDocumentResult = builder.objectRef<{
  data: Uint8Array;
  filename: string;
  mimeType: string;
}>('ExportDocumentResult');

ExportDocumentResult.implement({
  fields: (t) => ({
    data: t.expose('data', { type: 'Binary' }),
    filename: t.exposeString('filename'),
    mimeType: t.exposeString('mimeType'),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  exportDocument: t.withAuth({ session: true }).fieldWithInput({
    type: ExportDocumentResult,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      format: t.input.field({ type: DocumentExportFormat }),
      layout: t.input.field({ type: ExportDocumentPageLayoutInput, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const document = await db
        .select()
        .from(Documents)
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrowWith(new NotFoundError()));

      const entity = await db.select().from(Entities).where(eq(Entities.id, document.entityId)).then(firstOrThrowWith(new NotFoundError()));

      if (entity.visibility === EntityVisibility.PRIVATE && entity.userId !== ctx.session.userId) {
        throw new NotFoundError();
      }

      const content = await db
        .select()
        .from(DocumentContents)
        .where(eq(DocumentContents.documentId, document.id))
        .then(firstOrThrowWith(new NotFoundError()));

      const title = document.title || '(제목 없음)';
      const filename = `${title}${document.subtitle ? ` - ${document.subtitle}` : ''}`;

      const needsFonts = input.format === 'EPUB' || input.format === 'PDF';
      const [user, fontFamilies] = await Promise.all([
        db.select({ name: Users.name }).from(Users).where(eq(Users.id, entity.userId)).then(firstOrThrow),
        needsFonts ? getDocumentFontFamilies(entity.userId) : undefined,
      ]);

      if (input.format !== 'EPUB' && !input.layout) {
        throw new TypieError({ code: 'invalid_input', message: 'layout is required for this format' });
      }

      switch (input.format) {
        case 'DOCX': {
          if (!input.layout) throw new TypieError({ code: 'invalid_input', message: 'layout is required for this format' });
          const data = await generateDocumentDocx({
            snapshot: content.snapshot,
            title,
            author: user.name,
            pageWidth: input.layout.pageWidth,
            pageHeight: input.layout.pageHeight,
            pageMarginTop: input.layout.pageMarginTop,
            pageMarginBottom: input.layout.pageMarginBottom,
            pageMarginLeft: input.layout.pageMarginLeft,
            pageMarginRight: input.layout.pageMarginRight,
          });
          return {
            data,
            filename: `${filename}.docx`,
            // spell-checker:disable-next-line
            mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
          };
        }

        case 'EPUB': {
          if (!fontFamilies) throw new TypieError({ code: 'invalid_input', message: 'fontFamilies is required for this format' });
          const data = await generateDocumentEpub({
            snapshot: content.snapshot,
            title,
            author: user.name,
            fontFamilies,
          });
          return { data, filename: `${filename}.epub`, mimeType: 'application/epub+zip' };
        }

        case 'HWP': {
          if (!input.layout) throw new TypieError({ code: 'invalid_input', message: 'layout is required for this format' });
          const fontNameMap = await buildFontNameMap(entity.userId);
          const data = await generateDocumentHwp({
            snapshot: content.snapshot,
            title,
            author: user.name,
            pageWidth: input.layout.pageWidth,
            pageHeight: input.layout.pageHeight,
            pageMarginTop: input.layout.pageMarginTop,
            pageMarginBottom: input.layout.pageMarginBottom,
            pageMarginLeft: input.layout.pageMarginLeft,
            pageMarginRight: input.layout.pageMarginRight,
            fontNameMap,
          });
          return { data, filename: `${filename}.hwp`, mimeType: 'application/x-hwp' };
        }

        case 'PDF': {
          if (!input.layout) throw new TypieError({ code: 'invalid_input', message: 'layout is required for this format' });
          if (!fontFamilies) throw new TypieError({ code: 'invalid_input', message: 'fontFamilies is required for this format' });
          const data = await generateDocumentPdf({
            snapshot: content.snapshot,
            title,
            author: user.name,
            fonts: fontFamilies,
            layout: {
              pageWidth: input.layout.pageWidth,
              pageHeight: input.layout.pageHeight,
              pageMarginTop: input.layout.pageMarginTop,
              pageMarginBottom: input.layout.pageMarginBottom,
              pageMarginLeft: input.layout.pageMarginLeft,
              pageMarginRight: input.layout.pageMarginRight,
            },
          });
          return { data, filename: `${filename}.pdf`, mimeType: 'application/pdf' };
        }
      }
    },
  }),
}));

/** familyName → [{ weight, fullName, bucket, key }] 매핑을 빌드한다 (기본 폰트 + 유저 업로드 폰트) */
async function buildFontNameMap(userId: string): Promise<Map<string, { weight: number; fullName: string; postScriptName: string }[]>> {
  const map = new Map<string, { weight: number; fullName: string; postScriptName: string }[]>();

  // 기본 폰트
  for (const family of DEFAULT_FONT_FAMILIES) {
    map.set(
      family.familyName,
      family.fonts.map((f) => ({
        weight: f.weight,
        fullName: f.name,
        postScriptName: f.postScriptName,
      })),
    );
  }

  // 유저 업로드 폰트
  const rows = await db
    .select({
      familyName: FontFamilies.familyName,
      weight: Fonts.weight,
      fullName: Fonts.fullName,
      postScriptName: Fonts.postScriptName,
    })
    .from(Fonts)
    .innerJoin(FontFamilies, eq(FontFamilies.id, Fonts.familyId))
    .where(eq(FontFamilies.userId, userId))
    .orderBy(asc(Fonts.weight));

  for (const row of rows) {
    if (!row.postScriptName) continue;
    let entries = map.get(row.familyName);
    if (!entries) {
      entries = [];
      map.set(row.familyName, entries);
    }
    entries.push({
      weight: row.weight,
      fullName: row.fullName ?? row.postScriptName,
      postScriptName: row.postScriptName,
    });
  }

  return map;
}
