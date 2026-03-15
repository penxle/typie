// cspell:ignore OEBPS
import JSZip from 'jszip';
import { createElement, Fragment } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { parseDocument } from '../core/document.ts';
import { traverse } from '../core/traverse.ts';
import { collectUsedFonts, loadFontFiles } from './fonts.ts';
import { generateContainerXml, generateContentOpf, generateNavXhtml } from './meta.ts';
import { epubVisitor } from './nodes.tsx.js';
import { generateStylesheet } from './styles.ts';
import { extFromFormat } from './utils.ts';
import type { ExportFontFamily } from '../core/types.ts';
import type { EpubConvertContext } from './nodes.tsx.js';

export type GenerateDocumentEpubParams = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  fonts: ExportFontFamily[];
};

export async function generateDocumentEpub(params: GenerateDocumentEpubParams): Promise<Uint8Array> {
  const { title, author, fonts } = params;

  const parsed = await parseDocument(params.snapshot);
  const { defaults } = parsed;

  // 폰트 수집 및 로딩
  const usedFonts = collectUsedFonts(parsed.nodes, defaults.fontFamily);
  const fontFiles = await loadFontFiles(usedFonts, fonts);

  // Visitor 패턴으로 HTML 렌더링
  const ctx: EpubConvertContext = { nodes: parsed.nodes };
  const bodyNodes = traverse(parsed, epubVisitor, ctx);
  const bodyHtml = renderToStaticMarkup(createElement(Fragment, null, ...bodyNodes));

  // CSS 생성
  const stylesheet = generateStylesheet(fontFiles, {
    fontFamily: defaults.fontFamily,
    fontSizePt: defaults.fontSizePt100 / 100,
    lineHeight: defaults.lineHeight,
    blockGapPx: defaults.blockGapPx,
    paragraphIndentPx: defaults.paragraphIndentPx,
  });

  // EPUB 아카이브 패킹
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
