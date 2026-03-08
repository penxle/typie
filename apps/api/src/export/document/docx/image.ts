import { AlignmentType, ImageRun, Paragraph } from 'docx';
import { loadImageAssets } from '../external';
import { convertPlaceholderTable } from './blocks';
import type { ImageAsset } from '../external';

// DOCX content width in CSS px (A4 ~= 595pt ≈ 793px, minus ~1 inch margins each side ≈ 601px)
const CONTENT_WIDTH_PX = 600;

type ImageNode = {
  type: 'image';
  id?: string;
  proportion: number;
};

export async function loadImages(imageIds: string[]): Promise<Map<string, ImageAsset>> {
  if (imageIds.length === 0) return new Map();
  return loadImageAssets(imageIds);
}

function mapFormat(format: string): 'jpg' | 'png' | 'gif' | 'bmp' {
  if (format === 'image/jpeg' || format === 'image/jpg') return 'jpg';
  if (format === 'image/gif') return 'gif';
  if (format === 'image/bmp') return 'bmp';
  return 'png';
}

export function convertImage(node: ImageNode, assets: Map<string, ImageAsset>): Paragraph | ReturnType<typeof convertPlaceholderTable> {
  if (!node.id) {
    return convertPlaceholderTable('[이미지]');
  }

  const asset = assets.get(node.id);
  if (!asset || asset.width <= 0 || asset.height <= 0) {
    return convertPlaceholderTable('[이미지를 불러올 수 없습니다]');
  }

  const displayWidth = CONTENT_WIDTH_PX * Math.min(node.proportion, 1);
  const displayHeight = displayWidth * (asset.height / asset.width);

  return new Paragraph({
    alignment: AlignmentType.CENTER,
    children: [
      new ImageRun({
        type: mapFormat(asset.format),
        data: asset.bytes,
        transformation: {
          width: Math.round(displayWidth),
          height: Math.round(displayHeight),
        },
      }),
    ],
  });
}
