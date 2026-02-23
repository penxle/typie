import { GetObjectCommand } from '@aws-sdk/client-s3';
import fontkit from '@pdf-lib/fontkit';
import { LineCapStyle, LineJoinStyle, PDFDocument, popGraphicsState, pushGraphicsState, rgb, setLineJoin } from 'pdf-lib';
import sharp from 'sharp';
import * as aws from '@/external/aws';
import { decompressZstd } from '@/utils/compression';
import { outlineTextToSvg } from '@/utils/font';
import type { PDFFont, PDFImage, PDFPage } from 'pdf-lib';
import type { ResolvedExternalImage } from './external';
import type { VectorExternalElement, VectorOp, VectorPage, VectorPathCommand, VectorTextOp } from './vector';

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

const drawVectorOp = (page: import('pdf-lib').PDFPage, op: VectorOp, pdfHeight: number): void => {
  const path = toSvgPath(op.path);
  if (!path) return;

  if (op.type === 'fillPath') {
    page.drawSvgPath(path, {
      x: 0,
      y: pdfHeight,
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
    y: pdfHeight,
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

const drawInvisibleTextOp = (page: PDFPage, op: VectorTextOp, pdfHeight: number, font: PDFFont): void => {
  if (!op.text) return;
  const size = op.size * CSS_PX_TO_PDF_PT;
  if (!Number.isFinite(size) || size <= 0) return;

  try {
    page.drawText(op.text, {
      x: toPdf(op.x),
      y: pdfHeight - toPdf(op.y),
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

const externalBlockLabel = (external: VectorExternalElement): string => {
  if (external.data.type === 'image') return '이미지';
  if (external.data.type === 'file') return '파일';
  if (external.data.type === 'embed') return '임베드';
  return '보관된 블록';
};

const unsupportedMessage = (external: VectorExternalElement): string => `지원되지 않는 블록입니다 (${externalBlockLabel(external)})`;

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
        console.warn('[pdf] failed to load placeholder font', err);
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
        console.warn('[pdf] failed to load hidden text font', err);
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
      console.warn(`[pdf] failed to outline placeholder label: ${message}`, err);
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

const drawUnsupportedLabel = async (page: PDFPage, external: VectorExternalElement, rect: PdfRect): Promise<void> => {
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

const drawUnsupportedExternal = async (page: PDFPage, external: VectorExternalElement, pdfHeight: number): Promise<void> => {
  const rect = toPdfRect(external.bounds, pdfHeight);
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

const embedExternalImage = async (pdfDoc: PDFDocument, asset: ResolvedExternalImage): Promise<PDFImage | null> => {
  try {
    if (asset.format === 'image/png') {
      return await pdfDoc.embedPng(asset.bytes);
    }
    if (asset.format === 'image/jpeg' || asset.format === 'image/jpg') {
      return await pdfDoc.embedJpg(asset.bytes);
    }

    const pngBytes = await sharp(asset.bytes, { failOn: 'none', limitInputPixels: false }).png().toBuffer();
    return await pdfDoc.embedPng(new Uint8Array(pngBytes));
  } catch (err) {
    console.warn(`[pdf] failed to embed image ${asset.id}`, err);
    return null;
  }
};

const resolveImageRenderSize = (
  bounds: VectorExternalElement['bounds'],
  proportion: number,
  asset: ResolvedExternalImage,
): { width: number; height: number } => {
  if (asset.width <= 0 || asset.height <= 0) {
    return { width: bounds.width, height: bounds.height };
  }

  const aspectRatio = asset.height / asset.width;
  const desiredWidth = Math.min(asset.width, bounds.width * proportion);
  const desiredHeight = desiredWidth * aspectRatio;

  if (desiredHeight <= bounds.height) {
    return { width: desiredWidth, height: desiredHeight };
  }

  const fittedHeight = bounds.height;
  const fittedWidth = fittedHeight / aspectRatio;
  return { width: Math.min(fittedWidth, bounds.width), height: fittedHeight };
};

const drawExternalImage = (
  page: PDFPage,
  image: PDFImage,
  external: VectorExternalElement,
  proportion: number,
  asset: ResolvedExternalImage,
  pdfHeight: number,
): boolean => {
  const size = resolveImageRenderSize(external.bounds, proportion, asset);
  if (size.width <= 0 || size.height <= 0) {
    return false;
  }

  const x = external.bounds.x + (external.bounds.width - size.width) / 2;
  const y = external.bounds.y;

  page.drawImage(image, {
    x: toPdf(x),
    y: pdfHeight - toPdf(y + size.height),
    width: toPdf(size.width),
    height: toPdf(size.height),
  });

  return true;
};

const drawExternalElement = async (
  page: PDFPage,
  pdfDoc: PDFDocument,
  external: VectorExternalElement,
  imageAssets: ReadonlyMap<string, ResolvedExternalImage>,
  embeddedImageCache: Map<string, PDFImage | null>,
  pdfHeight: number,
): Promise<void> => {
  if (external.data.type !== 'image') {
    await drawUnsupportedExternal(page, external, pdfHeight);
    return;
  }

  if (!external.data.id) {
    await drawUnsupportedExternal(page, external, pdfHeight);
    return;
  }

  const asset = imageAssets.get(external.data.id);
  if (!asset) {
    await drawUnsupportedExternal(page, external, pdfHeight);
    return;
  }

  let embeddedImage = embeddedImageCache.get(asset.id);
  if (embeddedImage === undefined) {
    embeddedImage = await embedExternalImage(pdfDoc, asset);
    embeddedImageCache.set(asset.id, embeddedImage);
  }

  if (!embeddedImage) {
    await drawUnsupportedExternal(page, external, pdfHeight);
    return;
  }

  if (!drawExternalImage(page, embeddedImage, external, external.data.proportion, asset, pdfHeight)) {
    await drawUnsupportedExternal(page, external, pdfHeight);
  }
};

export async function createPdfFromVectorPages(
  pages: VectorPage[],
  title: string,
  author: string,
  imageAssets: ReadonlyMap<string, ResolvedExternalImage> = new Map(),
): Promise<Uint8Array> {
  const pdfDoc = await PDFDocument.create();
  const embeddedImageCache = new Map<string, PDFImage | null>();
  let hiddenTextFont: PDFFont | null = null;
  const hiddenTextFontData = await getHiddenTextFontData();
  if (hiddenTextFontData) {
    try {
      pdfDoc.registerFontkit(fontkit);
      hiddenTextFont = await pdfDoc.embedFont(hiddenTextFontData, { subset: true });
    } catch (err) {
      console.warn('[pdf] failed to embed hidden text font', err);
    }
  }

  pdfDoc.setTitle(title);
  pdfDoc.setAuthor(author);
  pdfDoc.setCreator('타이피 (https://typie.co)');
  pdfDoc.setProducer('타이피 (https://typie.co)');

  for (const pageData of pages) {
    const pdfWidth = pageData.width * CSS_PX_TO_PDF_PT;
    const pdfHeight = pageData.height * CSS_PX_TO_PDF_PT;
    const page = pdfDoc.addPage([pdfWidth, pdfHeight]);

    for (const op of pageData.ops) {
      drawVectorOp(page, op, pdfHeight);
    }

    for (const external of pageData.externalElements) {
      await drawExternalElement(page, pdfDoc, external, imageAssets, embeddedImageCache, pdfHeight);
    }

    if (hiddenTextFont) {
      for (const textOp of pageData.textOps) {
        drawInvisibleTextOp(page, textOp, pdfHeight, hiddenTextFont);
      }
    }
  }

  return pdfDoc.save();
}
