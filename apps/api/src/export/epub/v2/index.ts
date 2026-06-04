// cspell:ignore OEBPS
import JSZip from 'jszip';
import { createElement, Fragment } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { parseDocumentV2 } from '../../core/v2/document.ts';
import { collectUsedFontsV2 } from '../../core/v2/fonts.ts';
import { traverseV2 } from '../../core/v2/traverse.ts';
import { loadFontFiles } from '../fonts.ts';
import { generateContainerXml, generateContentOpf, generateNavXhtml } from '../meta.ts';
import { generateStylesheet } from '../styles.ts';
import { extFromFormat } from '../utils.ts';
import { epubVisitorV2 } from './nodes.tsx.js';
import type { ExportFontFamily } from '../../core/types.ts';
import type { EpubConvertContext } from './nodes.tsx.js';

export type GenerateDocumentEpubV2Params = {
  graph: Uint8Array;
  title: string;
  author: string;
  fonts: ExportFontFamily[];
};

export async function generateDocumentEpubV2(params: GenerateDocumentEpubV2Params): Promise<Uint8Array> {
  const { title, author, fonts } = params;

  const parsed = await parseDocumentV2(params.graph);
  const { defaults } = parsed;

  const usedFonts = collectUsedFontsV2(parsed.plain, defaults);
  const fontFiles = await loadFontFiles(usedFonts, fonts);

  const ctx: EpubConvertContext = {};
  const bodyNodes = traverseV2(parsed, epubVisitorV2, ctx);
  const bodyHtml = renderToStaticMarkup(createElement(Fragment, null, ...bodyNodes));

  const stylesheet = generateStylesheet(fontFiles, {
    fontFamily: defaults.fontFamily,
    fontSizePt: defaults.fontSizePt100 / 100,
    lineHeight: defaults.lineHeight,
    blockGapPx: defaults.blockGapPx,
    paragraphIndentPx: defaults.paragraphIndentPx,
  });

  const documentXhtml = `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>${escapeXml(title)}</title>
  <link rel="stylesheet" type="text/css" href="style.css"/>
</head>
<body>
${bodyHtml}
</body>
</html>`;

  const imagesMeta = [...parsed.images.entries()].map(([id, asset]) => {
    const ext = extFromFormat(asset.format);
    return { id, filename: `${id}.${ext}`, mediaType: asset.format, bytes: asset.bytes };
  });

  const fontsMeta = [...fontFiles.values()].map((f) => ({ filename: f.filename }));

  const zip = new JSZip();
  zip.file('mimetype', 'application/epub+zip', { compression: 'STORE' });
  zip.file('META-INF/container.xml', generateContainerXml());
  zip.file('OEBPS/content.opf', generateContentOpf({ title, author, images: imagesMeta, fonts: fontsMeta }));
  zip.file('OEBPS/nav.xhtml', generateNavXhtml(title));
  zip.file('OEBPS/style.css', stylesheet);
  zip.file('OEBPS/document.xhtml', documentXhtml);

  for (const img of imagesMeta) {
    zip.file(`OEBPS/images/${img.filename}`, img.bytes);
  }

  for (const font of fontFiles.values()) {
    zip.file(`OEBPS/fonts/${font.filename}`, font.bytes);
  }

  return zip.generateAsync({ type: 'uint8array' });
}

function escapeXml(str: string): string {
  return str.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;').replaceAll('"', '&quot;');
}
