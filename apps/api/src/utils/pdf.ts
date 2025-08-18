import { execFile } from 'node:child_process';
import { promises as fs } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

async function mergePDFsWithPdfLib(pdfBuffers: Buffer[]): Promise<Buffer> {
  const { PDFDocument } = await import('pdf-lib');
  const mergedPdf = await PDFDocument.create();

  for (const pdfBuffer of pdfBuffers) {
    const pdf = await PDFDocument.load(pdfBuffer);
    const pages = await mergedPdf.copyPages(pdf, pdf.getPageIndices());
    for (const page of pages) {
      mergedPdf.addPage(page);
    }
  }

  return Buffer.from(await mergedPdf.save());
}

export async function mergePDFs(pdfBuffers: Buffer[]): Promise<Buffer> {
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

    await Promise.all(pdfBuffers.map((pdfBuffer, i) => fs.writeFile(inputPaths[i], pdfBuffer)));

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

    await execFileAsync('gs', args);

    const optimizedPdfBuffer = await fs.readFile(outputPath);
    return optimizedPdfBuffer;
  } finally {
    await fs.rm(tempDir, { recursive: true, force: true }).catch(() => {
      // ignore cleanup errors
    });
  }
}
