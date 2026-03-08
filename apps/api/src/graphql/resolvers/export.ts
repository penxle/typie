import { eq } from 'drizzle-orm';
import { db, DocumentContents, Documents, Entities, firstOrThrow, firstOrThrowWith, TableCode, Users, validateDbId } from '@/db';
import { EntityVisibility } from '@/enums';
import { NotFoundError } from '@/errors';
import { generateDocumentDocx, generateDocumentEpub, generateDocumentPdf } from '@/export/document';
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

const ExportDocumentAsDocxResult = builder.objectRef<{
  data: Uint8Array;
  filename: string;
}>('ExportDocumentAsDocxResult');

ExportDocumentAsDocxResult.implement({
  fields: (t) => ({
    data: t.expose('data', { type: 'Binary' }),
    filename: t.exposeString('filename'),
  }),
});

const ExportDocumentAsEpubResult = builder.objectRef<{
  data: Uint8Array;
  filename: string;
}>('ExportDocumentAsEpubResult');

ExportDocumentAsEpubResult.implement({
  fields: (t) => ({
    data: t.expose('data', { type: 'Binary' }),
    filename: t.exposeString('filename'),
  }),
});

/**
 * * Mutations
 */

builder.mutationFields((t) => ({
  exportDocumentAsDocx: t.withAuth({ session: true }).fieldWithInput({
    type: ExportDocumentAsDocxResult,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
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

      const user = await db.select({ name: Users.name }).from(Users).where(eq(Users.id, entity.userId)).then(firstOrThrow);

      const docxBuffer = await generateDocumentDocx({
        snapshot: content.snapshot,
        title: document.title || '(제목 없음)',
        author: user.name,
      });

      return {
        data: docxBuffer,
        filename: `${document.title || '(제목 없음)'}${document.subtitle ? ` - ${document.subtitle}` : ''}.docx`,
      };
    },
  }),

  exportDocumentAsEpub: t.withAuth({ session: true }).fieldWithInput({
    type: ExportDocumentAsEpubResult,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
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

      const [user, fontFamilies] = await Promise.all([
        db.select({ name: Users.name }).from(Users).where(eq(Users.id, entity.userId)).then(firstOrThrow),
        getDocumentFontFamilies(entity.userId),
      ]);

      const epubBuffer = await generateDocumentEpub({
        snapshot: content.snapshot,
        title: document.title || '(제목 없음)',
        author: user.name,
        fontFamilies,
      });

      return {
        data: epubBuffer,
        filename: `${document.title || '(제목 없음)'}${document.subtitle ? ` - ${document.subtitle}` : ''}.epub`,
      };
    },
  }),

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
