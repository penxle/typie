import fs from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';

async function mergePDFsWithPdfLib(pdfBuffers: Uint8Array[]): Promise<Uint8Array> {
  const { PDFDocument } = await import('pdf-lib');
  const mergedPdf = await PDFDocument.create();

  for (const pdfBuffer of pdfBuffers) {
    const pdf = await PDFDocument.load(pdfBuffer);
    const pages = await mergedPdf.copyPages(pdf, pdf.getPageIndices());
    for (const page of pages) {
      mergedPdf.addPage(page);
    }
  }

  return await mergedPdf.save();
}

export async function mergePDFs(pdfBuffers: Uint8Array[]): Promise<Uint8Array> {
  if (pdfBuffers.length === 0) {
    throw new Error('Implementation error: At least one PDF buffer is required');
  }

  if (pdfBuffers.length === 1) {
    return pdfBuffers[0];
  }

  const isLocal = process.env.NODE_ENV === undefined;
  if (isLocal) {
    return mergePDFsWithPdfLib(pdfBuffers);
  }

  const tempDir = await fs.mkdtemp(path.join(tmpdir(), 'pdf-merge-'));

  try {
    const outputPath = path.join(tempDir, 'output.pdf');
    const inputPaths = pdfBuffers.map((_, i) => path.join(tempDir, `page-${i}.pdf`));

    await Promise.all(pdfBuffers.map((pdfBuffer, i) => Bun.write(inputPaths[i], pdfBuffer)));

    // NOTE: ghostscript 로 병합 및 최적화
    const args = [
      '-sDEVICE=pdfwrite',
      '-dCompatibilityLevel=1.4',
      '-dPDFSETTINGS=/default',
      // cspell:ignore NOPAUSE
      '-dNOPAUSE',
      '-dBATCH',
      '-dCompressFonts=true',
      '-dSubsetFonts=true',
      `-sOutputFile=${outputPath}`,
      ...inputPaths,
    ];

    const process = Bun.spawn(['gs', ...args]);
    await process.exited;

    return await Bun.file(outputPath).bytes();
  } finally {
    await fs.rm(tempDir, { recursive: true, force: true }).catch(() => {
      // ignore cleanup errors
    });
  }
}
