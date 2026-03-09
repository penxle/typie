import { GetObjectCommand } from '@aws-sdk/client-s3';
import fontkit from '@pdf-lib/fontkit';
import * as Sentry from '@sentry/bun';
import { LineCapStyle, LineJoinStyle, PDFDocument, popGraphicsState, pushGraphicsState, rgb, setLineJoin } from 'pdf-lib';
import sharp from 'sharp';
import * as aws from '@/external/aws';
import { decompressZstd } from '@/utils/compression';
import { outlineTextToSvg } from '@/utils/font';
import { computeDesiredSize } from '../external';
import type { PDFFont, PDFImage, PDFPage } from 'pdf-lib';
import type { Asset, ImageAsset } from '../external';
import type { VectorOp, VectorPage, VectorPathCommand, VectorTextOp } from './codec';
import type { ExternalElement } from './slate';

const CSS_PX_TO_PDF_PT = 72 / 96;
const PLACEHOLDER_BACKGROUND = rgb(0.965, 0.965, 0.969);
const PLACEHOLDER_BORDER = rgb(0.831, 0.831, 0.847);
const PLACEHOLDER_HATCH = rgb(0.78, 0.78, 0.8);
const PLACEHOLDER_LABEL_COLOR = rgb(0.329, 0.329, 0.361);
const PLACEHOLDER_HATCH_STEP = 12 * CSS_PX_TO_PDF_PT;
const PLACEHOLDER_HATCH_WIDTH = 0.5;
const PLACEHOLDER_LABEL_PADDING_RATIO = 0.14;
const PLACEHOLDER_LABEL_HEIGHT_RATIO = 0.35;
const PLACEHOLDER_FONT_BUCKET = 'typie-cdn';
const PLACEHOLDER_FONT_KEY = 'editor/fonts/Pretendard-Regular/original.bin';
const HIDDEN_TEXT_FONT_BUCKET = 'typie-cdn';
const HIDDEN_TEXT_FONT_KEY = 'fonts/editor/Noto-Phantom.ttf';
const EPSILON = 1e-3;

type PdfRect = { x: number; y: number; width: number; height: number };

type OutlinedPlaceholderLabel = {
  viewBox: {
    minX: number;
    minY: number;
    width: number;
    height: number;
  };
  paths: string[];
};

let placeholderFontDataPromise: Promise<Uint8Array | null> | null = null;
let hiddenTextFontDataPromise: Promise<Uint8Array | null> | null = null;
const outlinedPlaceholderLabelPromises = new Map<string, Promise<OutlinedPlaceholderLabel | null>>();

const toRgb = (color: [number, number, number, number]) => rgb(color[0] / 255, color[1] / 255, color[2] / 255);

const toOpacity = (color: [number, number, number, number]) => color[3] / 255;

const toPdf = (value: number): number => value * CSS_PX_TO_PDF_PT;

const toLineCap = (value: string | null | undefined): LineCapStyle | undefined => {
  if (value === 'round') return LineCapStyle.Round;
  if (value === 'square') return LineCapStyle.Projecting;
  if (value === 'butt') return LineCapStyle.Butt;
  return undefined;
};

const toLineJoin = (value: string | null | undefined): LineJoinStyle | undefined => {
  if (value === 'round') return LineJoinStyle.Round;
  if (value === 'bevel') return LineJoinStyle.Bevel;
  if (value === 'miter') return LineJoinStyle.Miter;
  return undefined;
};

const formatNumber = (value: number): string => Number(value.toFixed(3)).toString();

const toSvgPath = (commands: VectorPathCommand[]): string => {
  const parts: string[] = [];

  for (const command of commands) {
    switch (command.type) {
      case 'moveTo': {
        parts.push(`M ${formatNumber(command.x)} ${formatNumber(command.y)}`);
        break;
      }
      case 'lineTo': {
        parts.push(`L ${formatNumber(command.x)} ${formatNumber(command.y)}`);
        break;
      }
      case 'quadTo': {
        parts.push(`Q ${formatNumber(command.cx)} ${formatNumber(command.cy)} ${formatNumber(command.x)} ${formatNumber(command.y)}`);
        break;
      }
      case 'cubicTo': {
        parts.push(
          `C ${formatNumber(command.c1x)} ${formatNumber(command.c1y)} ${formatNumber(command.c2x)} ${formatNumber(command.c2y)} ${formatNumber(command.x)} ${formatNumber(command.y)}`,
        );
        break;
      }
      case 'closePath': {
        parts.push('Z');
        break;
      }
    }
  }

  return parts.join(' ');
};

const drawVectorOp = (ctx: PdfContext, page: PDFPage, op: VectorOp): void => {
  const path = toSvgPath(op.path);
  if (!path) return;

  if (op.type === 'fillPath') {
    page.drawSvgPath(path, {
      x: 0,
      y: ctx.pageHeight,
      scale: CSS_PX_TO_PDF_PT,
      color: toRgb(op.color),
      opacity: toOpacity(op.color),
    });
    return;
  }

  const lineJoin = toLineJoin(op.lineJoin);
  if (lineJoin !== undefined) {
    page.pushOperators(pushGraphicsState(), setLineJoin(lineJoin));
  }

  page.drawSvgPath(path, {
    x: 0,
    y: ctx.pageHeight,
    scale: CSS_PX_TO_PDF_PT,
    borderColor: toRgb(op.color),
    borderOpacity: toOpacity(op.color),
    borderWidth: op.width * CSS_PX_TO_PDF_PT,
    borderLineCap: toLineCap(op.lineCap),
  });

  if (lineJoin !== undefined) {
    page.pushOperators(popGraphicsState());
  }
};

const drawInvisibleTextOp = (ctx: PdfContext, page: PDFPage, op: VectorTextOp, font: PDFFont): void => {
  if (!op.text) return;
  const size = op.size * CSS_PX_TO_PDF_PT;
  if (!Number.isFinite(size) || size <= 0) return;

  try {
    page.drawText(op.text, {
      x: toPdf(op.x),
      y: ctx.pageHeight - toPdf(op.y),
      size,
      font,
      color: rgb(0, 0, 0),
      opacity: 0,
    });
  } catch {
    // Ignore encoding failures for hidden text layer.
  }
};

const toPdfRect = (bounds: { x: number; y: number; width: number; height: number }, pdfHeight: number): PdfRect => ({
  x: toPdf(bounds.x),
  y: pdfHeight - toPdf(bounds.y + bounds.height),
  width: toPdf(bounds.width),
  height: toPdf(bounds.height),
});

const externalBlockLabel = (external: ExternalElement): string => {
  if (external.data.type === 'image') return '이미지';
  if (external.data.type === 'file') return '파일';
  if (external.data.type === 'embed') return '임베드';
  return '보관된 블록';
};

const unsupportedMessage = (external: ExternalElement): string => `지원되지 않는 블록입니다 (${externalBlockLabel(external)})`;

const parseOutlinedPlaceholderSvg = (svg: string): OutlinedPlaceholderLabel | null => {
  const viewBoxMatch = svg.match(/viewBox="([^"]+)"/);
  if (!viewBoxMatch) return null;

  const [minX, minY, width, height] = viewBoxMatch[1].split(/\s+/).map((value) => Number.parseFloat(value));
  if (![minX, minY, width, height].every(Number.isFinite)) return null;
  if (width <= 0 || height <= 0) return null;

  const paths = [...svg.matchAll(/<path[^>]*\sd="([^"]+)"/g)].map((match) => match[1]).filter(Boolean);
  if (paths.length === 0) return null;

  return {
    viewBox: { minX, minY, width, height },
    paths,
  };
};

const getPlaceholderFontData = (): Promise<Uint8Array | null> => {
  if (!placeholderFontDataPromise) {
    placeholderFontDataPromise = (async () => {
      try {
        const object = await aws.s3.send(
          new GetObjectCommand({
            Bucket: PLACEHOLDER_FONT_BUCKET,
            Key: PLACEHOLDER_FONT_KEY,
          }),
        );

        if (!object.Body) {
          return null;
        }

        const compressed = await object.Body.transformToByteArray();
        return await decompressZstd(compressed);
      } catch (err) {
        Sentry.captureException(err);
        return null;
      }
    })();
  }

  return placeholderFontDataPromise;
};

const getHiddenTextFontData = (): Promise<Uint8Array | null> => {
  if (!hiddenTextFontDataPromise) {
    hiddenTextFontDataPromise = (async () => {
      try {
        const object = await aws.s3.send(
          new GetObjectCommand({
            Bucket: HIDDEN_TEXT_FONT_BUCKET,
            Key: HIDDEN_TEXT_FONT_KEY,
          }),
        );

        if (!object.Body) {
          return null;
        }

        return new Uint8Array(await object.Body.transformToByteArray());
      } catch (err) {
        Sentry.captureException(err);
        return null;
      }
    })();
  }

  return hiddenTextFontDataPromise;
};

const getOutlinedPlaceholderLabel = (message: string): Promise<OutlinedPlaceholderLabel | null> => {
  const cached = outlinedPlaceholderLabelPromises.get(message);
  if (cached) {
    return cached;
  }

  const promise = (async () => {
    const fontData = await getPlaceholderFontData();
    if (!fontData) {
      return null;
    }

    try {
      const svg = await outlineTextToSvg(fontData, message);
      return parseOutlinedPlaceholderSvg(svg);
    } catch (err) {
      Sentry.captureException(err);
      return null;
    }
  })();

  outlinedPlaceholderLabelPromises.set(message, promise);
  return promise;
};

const addSegmentPoint = (points: { x: number; y: number }[], point: { x: number; y: number }, rect: PdfRect): void => {
  const x0 = rect.x;
  const y0 = rect.y;
  const x1 = rect.x + rect.width;
  const y1 = rect.y + rect.height;

  if (point.x < x0 - EPSILON || point.x > x1 + EPSILON || point.y < y0 - EPSILON || point.y > y1 + EPSILON) {
    return;
  }

  const duplicate = points.some((v) => Math.abs(v.x - point.x) <= EPSILON && Math.abs(v.y - point.y) <= EPSILON);
  if (!duplicate) {
    points.push({
      x: Math.min(Math.max(point.x, x0), x1),
      y: Math.min(Math.max(point.y, y0), y1),
    });
  }
};

const hatchSegmentForIntercept = (rect: PdfRect, c: number): { start: { x: number; y: number }; end: { x: number; y: number } } | null => {
  const x0 = rect.x;
  const y0 = rect.y;
  const x1 = rect.x + rect.width;
  const y1 = rect.y + rect.height;

  const points: { x: number; y: number }[] = [];
  addSegmentPoint(points, { x: x0, y: x0 + c }, rect);
  addSegmentPoint(points, { x: x1, y: x1 + c }, rect);
  addSegmentPoint(points, { x: y0 - c, y: y0 }, rect);
  addSegmentPoint(points, { x: y1 - c, y: y1 }, rect);

  if (points.length < 2) {
    return null;
  }

  let bestStart = points[0];
  let bestEnd = points[1];
  let bestDistance = -1;

  for (let i = 0; i < points.length; i++) {
    for (let j = i + 1; j < points.length; j++) {
      const dx = points[i].x - points[j].x;
      const dy = points[i].y - points[j].y;
      const distance = dx * dx + dy * dy;
      if (distance > bestDistance) {
        bestDistance = distance;
        bestStart = points[i];
        bestEnd = points[j];
      }
    }
  }

  return {
    start: bestStart,
    end: bestEnd,
  };
};

const drawUniformHatch = (page: PDFPage, rect: PdfRect): void => {
  const x0 = rect.x;
  const y0 = rect.y;
  const x1 = rect.x + rect.width;
  const y1 = rect.y + rect.height;

  const cMin = y0 - x1;
  const cMax = y1 - x0;
  let c = Math.floor(cMin / PLACEHOLDER_HATCH_STEP) * PLACEHOLDER_HATCH_STEP;

  while (c <= cMax + EPSILON) {
    const segment = hatchSegmentForIntercept(rect, c);
    if (segment) {
      page.drawLine({
        start: segment.start,
        end: segment.end,
        color: PLACEHOLDER_HATCH,
        thickness: PLACEHOLDER_HATCH_WIDTH,
        opacity: 0.7,
      });
    }
    c += PLACEHOLDER_HATCH_STEP;
  }
};

const drawUnsupportedLabel = async (page: PDFPage, external: ExternalElement, rect: PdfRect): Promise<void> => {
  const outlined = await getOutlinedPlaceholderLabel(unsupportedMessage(external));
  if (!outlined) {
    return;
  }

  const maxWidth = rect.width * (1 - PLACEHOLDER_LABEL_PADDING_RATIO * 2);
  const maxHeight = rect.height * PLACEHOLDER_LABEL_HEIGHT_RATIO;
  if (maxWidth <= 0 || maxHeight <= 0) {
    return;
  }

  const scale = Math.min(maxWidth / outlined.viewBox.width, maxHeight / outlined.viewBox.height);
  if (!Number.isFinite(scale) || scale <= 0) {
    return;
  }

  const drawWidth = outlined.viewBox.width * scale;
  const drawHeight = outlined.viewBox.height * scale;
  const left = rect.x + (rect.width - drawWidth) / 2;
  const top = rect.y + (rect.height + drawHeight) / 2;

  const x = left - outlined.viewBox.minX * scale;
  const y = top + outlined.viewBox.minY * scale;

  for (const path of outlined.paths) {
    page.drawSvgPath(path, {
      x,
      y,
      scale,
      color: PLACEHOLDER_LABEL_COLOR,
      opacity: 0.95,
    });
  }
};

const drawUnsupportedExternal = async (ctx: PdfContext, page: PDFPage, external: ExternalElement): Promise<void> => {
  const rect = toPdfRect(external.bounds, ctx.pageHeight);
  if (rect.width <= 0 || rect.height <= 0) return;

  page.drawRectangle({
    x: rect.x,
    y: rect.y,
    width: rect.width,
    height: rect.height,
    color: PLACEHOLDER_BACKGROUND,
    borderColor: PLACEHOLDER_BORDER,
    borderWidth: 0.75,
  });

  drawUniformHatch(page, rect);
  await drawUnsupportedLabel(page, external, rect);
};

type PdfContext = {
  doc: PDFDocument;
  assets: ReadonlyMap<string, Asset>;
  embeddedImages: Map<string, PDFImage | null>;
  pageWidth: number;
  pageHeight: number;
};

const embedExternalImage = async (ctx: PdfContext, asset: ImageAsset): Promise<PDFImage | null> => {
  try {
    if (asset.format === 'image/png') {
      return await ctx.doc.embedPng(asset.bytes);
    }

    if (asset.format === 'image/jpeg' || asset.format === 'image/jpg') {
      return await ctx.doc.embedJpg(asset.bytes);
    }

    const bytes = await sharp(asset.bytes, { failOn: 'none', limitInputPixels: false }).png().toBuffer();
    return await ctx.doc.embedPng(new Uint8Array(bytes));
  } catch (err) {
    Sentry.captureException(err);
    return null;
  }
};

const drawExternalImage = (ctx: PdfContext, page: PDFPage, image: PDFImage, external: ExternalElement, asset: ImageAsset): boolean => {
  const { width, height } = computeDesiredSize(external, asset);
  if (width <= 0 || height <= 0) return false;

  const x = external.bounds.x + (external.bounds.width - width) / 2;
  const y = external.bounds.y;

  page.drawImage(image, {
    x: toPdf(x),
    y: ctx.pageHeight - toPdf(y + height),
    width: toPdf(width),
    height: toPdf(height),
  });

  return true;
};

const drawExternalElement = async (ctx: PdfContext, page: PDFPage, external: ExternalElement): Promise<void> => {
  if (external.data.type !== 'image') {
    await drawUnsupportedExternal(ctx, page, external);
    return;
  }

  const asset = ctx.assets.get(external.nodeId);
  if (!asset) {
    await drawUnsupportedExternal(ctx, page, external);
    return;
  }

  let embeddedImage = ctx.embeddedImages.get(asset.id);
  if (embeddedImage === undefined) {
    embeddedImage = await embedExternalImage(ctx, asset);
    ctx.embeddedImages.set(asset.id, embeddedImage);
  }

  if (!embeddedImage) {
    await drawUnsupportedExternal(ctx, page, external);
    return;
  }

  if (!drawExternalImage(ctx, page, embeddedImage, external, asset)) {
    await drawUnsupportedExternal(ctx, page, external);
  }
};

export async function createPdfFromVectorPages(
  pages: VectorPage[],
  title: string,
  author: string,
  externals: readonly ExternalElement[],
  assets: ReadonlyMap<string, Asset>,
): Promise<Uint8Array> {
  const doc = await PDFDocument.create();
  const firstPage = pages[0];
  if (!firstPage) {
    return doc.save();
  }

  const ctx: PdfContext = {
    doc,
    assets,
    embeddedImages: new Map(),
    pageWidth: firstPage.width * CSS_PX_TO_PDF_PT,
    pageHeight: firstPage.height * CSS_PX_TO_PDF_PT,
  };

  let hiddenTextFont: PDFFont | null = null;
  const hiddenTextFontData = await getHiddenTextFontData();
  if (hiddenTextFontData) {
    try {
      doc.registerFontkit(fontkit);
      hiddenTextFont = await doc.embedFont(hiddenTextFontData, { subset: true });
    } catch (err) {
      console.warn('[pdf] failed to embed hidden text font', err);
    }
  }

  doc.setTitle(title);
  doc.setAuthor(author);
  doc.setCreator('타이피 (https://typie.co)');
  doc.setProducer('타이피 (https://typie.co)');

  for (const [i, pageData] of pages.entries()) {
    const page = doc.addPage([ctx.pageWidth, ctx.pageHeight]);

    for (const op of pageData.ops) {
      drawVectorOp(ctx, page, op);
    }

    for (const external of externals) {
      if (external.pageIdx !== i) continue;
      await drawExternalElement(ctx, page, external);
    }

    if (hiddenTextFont) {
      for (const textOp of pageData.textOps) {
        drawInvisibleTextOp(ctx, page, textOp, hiddenTextFont);
      }
    }
  }

  return doc.save();
}
