import type { ExportFormat, ExportOptions } from './core/types.ts';

export type { ExportFont, ExportFontFamily, ExportFormat, ExportOptions, PageLayout } from './core/types.ts';

export async function generateDocument(format: ExportFormat, options: ExportOptions): Promise<Uint8Array> {
  switch (format) {
    case 'hwp': {
      const { generateDocumentHwp } = await import('./hwp/index.ts');
      if (!options.layout) throw new Error('layout is required for HWP');
      return generateDocumentHwp({
        snapshot: options.snapshot,
        title: options.title,
        author: options.author,
        fonts: options.fonts,
        ...options.layout,
      });
    }
    case 'docx': {
      const { generateDocumentDocx } = await import('./docx/index.ts');
      if (!options.layout) throw new Error('layout is required for DOCX');
      return generateDocumentDocx({
        snapshot: options.snapshot,
        title: options.title,
        author: options.author,
        fonts: options.fonts,
        ...options.layout,
      });
    }
    case 'epub': {
      const { generateDocumentEpub } = await import('./epub/index.ts');
      return generateDocumentEpub({
        snapshot: options.snapshot,
        title: options.title,
        author: options.author,
        fonts: options.fonts,
      });
    }
    case 'pdf': {
      const { generateDocumentPdf } = await import('./pdf/generate.ts');
      if (!options.layout) throw new Error('layout is required for PDF');
      return generateDocumentPdf({
        snapshot: options.snapshot,
        title: options.title,
        author: options.author,
        fonts: options.fonts.map((f) => ({
          familyName: f.family,
          fonts: f.weights.map((w) => ({ weight: w.weight, url: w.url })),
        })),
        layout: options.layout,
      });
    }
  }
}
