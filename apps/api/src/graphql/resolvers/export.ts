import { eq } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, firstOrThrow, firstOrThrowWith, TableCode, Users, validateDbId } from '@/db';
import { EntityVisibility } from '@/enums';
import { NotFoundError } from '@/errors';
import { generateDocumentPdf } from '@/export/document';
import { getDocumentFontFamilies } from '@/utils/document';
import { builder } from '../builder';

/**
 * * Types
 */

const ExportDocumentAsPdfResult = builder.objectRef<{
  data: Uint8Array;
  filename: string;
}>('ExportDocumentAsPdfResult');

ExportDocumentAsPdfResult.implement({
  fields: (t) => ({
    data: t.expose('data', { type: 'Binary' }),
    filename: t.exposeString('filename'),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  exportDocumentAsPdf: t.withAuth({ session: true }).fieldWithInput({
    type: ExportDocumentAsPdfResult,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      pageWidth: t.input.int(),
      pageHeight: t.input.int(),
      pageMarginTop: t.input.int(),
      pageMarginBottom: t.input.int(),
      pageMarginLeft: t.input.int(),
      pageMarginRight: t.input.int(),
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

      const [user, fonts] = await Promise.all([
        db.select({ name: Users.name }).from(Users).where(eq(Users.id, entity.userId)).then(firstOrThrow),
        getDocumentFontFamilies(entity.userId),
      ]);

      const pdfBuffer = await generateDocumentPdf({
        snapshot: content.snapshot,
        title: document.title || '(제목 없음)',
        author: user.name,
        fonts,
        layout: {
          pageWidth: input.pageWidth,
          pageHeight: input.pageHeight,
          pageMarginTop: input.pageMarginTop,
          pageMarginBottom: input.pageMarginBottom,
          pageMarginLeft: input.pageMarginLeft,
          pageMarginRight: input.pageMarginRight,
        },
      });

      return {
        data: pdfBuffer,
        filename: `${document.title || '(제목 없음)'}${document.subtitle ? ` - ${document.subtitle}` : ''}.pdf`,
      };
    },
  }),
}));
