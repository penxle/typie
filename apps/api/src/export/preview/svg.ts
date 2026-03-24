import sharp from 'sharp';
import { computeDesiredSize } from '../core/external.ts';
import type { VectorOp, VectorPage, VectorPathCommand } from '../core/codec.ts';
import type { Asset } from '../core/external.ts';
import type { ExternalElement } from '../core/slate.ts';

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

const toRgba = (color: [number, number, number, number]): string => {
  const [r, g, b, a] = color;
  return a === 255 ? `rgb(${r},${g},${b})` : `rgba(${r},${g},${b},${(a / 255).toFixed(3)})`;
};

const vectorOpToSvg = (op: VectorOp): string => {
  const d = toSvgPath(op.path);
  if (!d) return '';

  if (op.type === 'fillPath') {
    const fillRule = op.fillRule === 'evenOdd' ? ' fill-rule="evenodd"' : '';
    return `<path d="${d}" fill="${toRgba(op.color)}"${fillRule}/>`;
  }

  return `<path d="${d}" fill="none" stroke="${toRgba(op.color)}" stroke-width="${formatNumber(op.width)}" stroke-linecap="${op.lineCap}" stroke-linejoin="${op.lineJoin}"/>`;
};

export type PageMargins = {
  top: number;
  bottom: number;
  left: number;
  right: number;
};

export async function buildPageSvg(
  page: VectorPage,
  externals: readonly ExternalElement[],
  assets: ReadonlyMap<string, Asset>,
  margins: PageMargins,
): Promise<string> {
  const elements: string[] = [];

  for (const op of page.ops) {
    const svg = vectorOpToSvg(op);
    if (svg) elements.push(svg);
  }

  for (const ext of externals) {
    if (ext.pageIdx !== 0) continue;
    if (ext.data.type !== 'image') continue;

    const asset = assets.get(ext.nodeId);
    if (!asset) continue;

    const { width, height } = computeDesiredSize(ext, asset);
    if (width <= 0 || height <= 0) continue;

    const x = ext.bounds.x + (ext.bounds.width - width) / 2;
    const y = ext.bounds.y;

    const pngBuffer = await sharp(asset.bytes, { failOn: 'none', limitInputPixels: false }).png().toBuffer();
    const base64 = Uint8Array.from(pngBuffer).toBase64();

    elements.push(
      `<image href="data:image/png;base64,${base64}" x="${formatNumber(x)}" y="${formatNumber(y)}" width="${formatNumber(width)}" height="${formatNumber(height)}"/>`,
    );
  }

  const contentWidth = page.width - margins.left - margins.right;
  const contentHeight = page.height - margins.top - margins.bottom;

  return `<svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" viewBox="${margins.left} ${margins.top} ${contentWidth} ${contentHeight}" width="${contentWidth}" height="${contentHeight}">${elements.join('')}</svg>`;
}
