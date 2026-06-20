import fontFamiliesJson from '@typie/assets/fonts.json' with { type: 'json' };
import { DocumentExportFormat, EntityVisibility } from '@typie/lib/enums';
import { NotFoundError, TypieError } from '@typie/lib/errors';
import { and, asc, eq, inArray } from 'drizzle-orm';
import {
  db,
  DocumentContents,
  Documents,
  DocumentStates,
  Entities,
  first,
  firstOrThrow,
  firstOrThrowWith,
  FontFamilies,
  FontNames,
  Fonts,
  TableCode,
  Users,
  validateDbId,
} from '#/db/index.ts';
import { generateDocument } from '#/export/index.ts';
import { assertActiveSubscription } from '#/utils/plan.ts';
import { builder } from '../builder.ts';
import type { ExportFontFamily, ExportFormat, PageLayout } from '#/export/index.ts';

type FontFamilyEntry = {
  source: 'DEFAULT' | 'FALLBACK';
  familyName: string;
  fonts: {
    weight: number;
    path: string;
    postScriptName: string;
    names: { nameId: number; platformId: number; languageId: number; value: string }[];
  }[];
};
const fontFamilies = fontFamiliesJson as unknown as FontFamilyEntry[];

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

// cspell:ignore wordprocessingml
const FORMAT_META: Record<ExportFormat, { ext: string; mimeType: string }> = {
  hwp: { ext: 'hwp', mimeType: 'application/x-hwp' },
  docx: { ext: 'docx', mimeType: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' },
  epub: { ext: 'epub', mimeType: 'application/epub+zip' },
  pdf: { ext: 'pdf', mimeType: 'application/pdf' },
};

builder.mutationFields((t) => ({
  exportDocument: t.withAuth({ session: true }).fieldWithInput({
    type: ExportDocumentResult,
    input: {
      documentId: t.input.id({ validate: validateDbId(TableCode.DOCUMENTS) }),
      format: t.input.field({ type: DocumentExportFormat }),
      layout: t.input.field({ type: ExportDocumentPageLayoutInput, required: false }),
    },
    resolve: async (_, { input }, ctx) => {
      const format = input.format.toLowerCase() as ExportFormat;
      const meta = FORMAT_META[format];

      const document = await db
        .select()
        .from(Documents)
        .where(eq(Documents.id, input.documentId))
        .then(firstOrThrowWith(new NotFoundError()));

      const entity = await db.select().from(Entities).where(eq(Entities.id, document.entityId)).then(firstOrThrowWith(new NotFoundError()));

      if (entity.visibility === EntityVisibility.PRIVATE && entity.userId !== ctx.session.userId) {
        throw new NotFoundError();
      }

      if (format !== 'pdf') {
        await assertActiveSubscription({ userId: ctx.session.userId });
      }

      const layout = input.layout ?? undefined;
      if (format !== 'epub' && !layout) {
        throw new TypieError({ code: 'invalid_input', message: 'layout is required for this format' });
      }

      const state = await db
        .select({ graph: DocumentStates.graph })
        .from(DocumentStates)
        .where(eq(DocumentStates.documentId, document.id))
        .then(first);

      const title = document.title || '(제목 없음)';
      const filename = `${title}${document.subtitle ? ` - ${document.subtitle}` : ''}`;

      const user = await db.select({ name: Users.name }).from(Users).where(eq(Users.id, entity.userId)).then(firstOrThrow);

      if (state) {
        let data: Uint8Array;

        if (format === 'pdf') {
          const { generateDocumentPdfV2 } = await import('../../export/pdf/v2/generate.ts');
          data = await generateDocumentPdfV2({
            graph: state.graph,
            userId: entity.userId,
            title,
            author: user.name,
            layout: layout as PageLayout,
          });
        } else {
          const fonts = await buildExportFonts(entity.userId);

          if (format === 'hwp') {
            const { generateDocumentHwpV2 } = await import('../../export/hwp/v2/index.ts');
            data = await generateDocumentHwpV2({ graph: state.graph, title, author: user.name, fonts, layout: layout as PageLayout });
          } else if (format === 'docx') {
            const { generateDocumentDocxV2 } = await import('../../export/docx/v2/index.ts');
            data = await generateDocumentDocxV2({ graph: state.graph, title, author: user.name, fonts, layout: layout as PageLayout });
          } else {
            const { generateDocumentEpubV2 } = await import('../../export/epub/v2/index.ts');
            data = await generateDocumentEpubV2({ graph: state.graph, title, author: user.name, fonts });
          }
        }

        return { data, filename: `${filename}.${meta.ext}`, mimeType: meta.mimeType };
      }

      const content = await db
        .select()
        .from(DocumentContents)
        .where(eq(DocumentContents.documentId, document.id))
        .then(firstOrThrowWith(new NotFoundError()));

      const fonts = await buildExportFonts(entity.userId);

      const data = await generateDocument(format, {
        snapshot: content.snapshot,
        title,
        author: user.name,
        fonts,
        layout: input.layout ?? undefined,
      });

      return { data, filename: `${filename}.${meta.ext}`, mimeType: meta.mimeType };
    },
  }),
}));

/** ExportFontFamily[] 빌드 (기본 폰트 + 유저 업로드 폰트) */
export async function buildExportFonts(userId: string): Promise<ExportFontFamily[]> {
  const families: ExportFontFamily[] = [];

  // 기본 폰트
  for (const family of fontFamilies) {
    if (family.source !== 'DEFAULT') continue;
    families.push({
      family: family.familyName,
      weights: family.fonts.map((f) => ({
        weight: f.weight,
        url: `https://cdn.typie.net/editor/fonts/${f.path}`,
        name: f.names.find((n) => n.nameId === 4)?.value ?? f.postScriptName,
        localizedName: f.names.find((n) => n.nameId === 1 && n.languageId === 0x04_12)?.value ?? f.names.find((n) => n.nameId === 1)?.value,
        postScriptName: f.postScriptName,
      })),
    });
  }

  // 유저 업로드 폰트
  const rows = await db
    .select({
      id: Fonts.id,
      familyName: FontFamilies.familyName,
      weight: Fonts.weight,
      path: Fonts.path,
      postScriptName: Fonts.postScriptName,
    })
    .from(Fonts)
    .innerJoin(FontFamilies, eq(FontFamilies.id, Fonts.familyId))
    .where(eq(FontFamilies.userId, userId))
    .orderBy(asc(Fonts.weight));

  if (rows.length > 0) {
    const fontIds = rows.map((r) => r.id);
    const nameRecords = await db
      .select({
        fontId: FontNames.fontId,
        nameId: FontNames.nameId,
        languageId: FontNames.languageId,
        value: FontNames.value,
      })
      .from(FontNames)
      .where(and(inArray(FontNames.fontId, fontIds), inArray(FontNames.nameId, [1, 4])));

    const faceMap = new Map<string, { faceName?: string; faceDefault?: string }>();
    for (const rec of nameRecords) {
      let entry = faceMap.get(rec.fontId);
      if (!entry) {
        entry = {};
        faceMap.set(rec.fontId, entry);
      }
      if (rec.nameId === 1) {
        if (rec.languageId === 0x04_12 || !entry.faceName) {
          entry.faceName = rec.value;
        }
      } else if (rec.nameId === 4 && !entry.faceDefault) {
        entry.faceDefault = rec.value;
      }
    }

    // familyName별로 그룹화
    const grouped = new Map<string, ExportFontFamily>();
    for (const row of rows) {
      let fam = grouped.get(row.familyName);
      if (!fam) {
        fam = { family: row.familyName, weights: [] };
        grouped.set(row.familyName, fam);
      }
      const face = faceMap.get(row.id);
      fam.weights.push({
        weight: row.weight,
        url: `https://typie.net/fonts/${row.path}`,
        name: face?.faceDefault ?? row.postScriptName,
        localizedName: face?.faceName,
        postScriptName: row.postScriptName,
      });
    }
    families.push(...grouped.values());
  }

  return families;
}
