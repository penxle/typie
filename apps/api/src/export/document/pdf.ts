import { PDFDocument } from 'pdf-lib';
import type { RenderPageResult } from './render';

const CSS_PX_TO_PDF_PT = 72 / 96;

export async function createPdfFromPages(
  pages: RenderPageResult[],
  scaleFactor: number,
  title: string,
  author: string,
): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.create();

  pdfDoc.setTitle(title);
  pdfDoc.setAuthor(author);
  pdfDoc.setCreator('타이피 (https://typie.co)');
  pdfDoc.setProducer('타이피 (https://typie.co)');

  for (const page of pages) {
    const pngImage = await pdfDoc.embedPng(page.png);

    const pdfWidth = (page.width / scaleFactor) * CSS_PX_TO_PDF_PT;
    const pdfHeight = (page.height / scaleFactor) * CSS_PX_TO_PDF_PT;

    const pdfPage = pdfDoc.addPage([pdfWidth, pdfHeight]);

    pdfPage.drawImage(pngImage, {
      x: 0,
      y: 0,
      width: pdfWidth,
      height: pdfHeight,
    });
  }

  return pdfDoc.save();
}
