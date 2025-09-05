import { createDefaultPageLayout } from '@typie/ui/utils';
import { AlignmentType, ImageRun, TextRun } from 'docx';
import sizeOf from 'image-size';
import { inchToPx, mmToInch } from '../utils/unit';
import { createParagraph } from '../utils/utils';
import type { JSONContent } from '@tiptap/core';
import type { Paragraph } from 'docx';
import type { ConvertOptions } from '../types';

export type ImageData = {
  buffer: Buffer;
  width: number;
  height: number;
  type: 'jpg' | 'png' | 'gif' | 'bmp'; // NOTE: RegularMediaType in docx
};

export type ImageCache = Map<string, ImageData | null>;

export async function downloadImage(url: string): Promise<ImageData | null> {
  const urlPath = new URL(url).pathname;
  const extension = urlPath.split('.').pop()?.toLowerCase() || 'png';
  const imageType = extension === 'jpeg' ? 'jpg' : extension;

  const supportedTypes = ['jpg', 'png', 'gif', 'bmp'];
  if (!supportedTypes.includes(imageType)) {
    console.error(`Unsupported image type: ${imageType}`);
    return null;
  }

  const response = await fetch(url);
  if (!response.ok) {
    console.error(`Failed to fetch image: ${response.statusText}`);
    return null;
  }

  const arrayBuffer = await response.arrayBuffer();
  const buffer = Buffer.from(arrayBuffer);

  const dimensions = sizeOf(new Uint8Array(buffer));
  const width = dimensions.width;
  const height = dimensions.height;

  return {
    buffer,
    width,
    height,
    type: imageType as 'jpg' | 'png' | 'gif' | 'bmp',
  };
}

export function extractImageUrls(content: JSONContent): Set<string> {
  const urls = new Set<string>();

  function traverse(node: JSONContent) {
    if (!node) return;

    if (node.type === 'image' && node.attrs?.url) {
      urls.add(node.attrs.url);
    }

    if (node.content && Array.isArray(node.content)) {
      for (const child of node.content) {
        traverse(child);
      }
    }
  }

  traverse(content);
  return urls;
}

export async function downloadAllImages(content: JSONContent): Promise<ImageCache> {
  const urls = extractImageUrls(content);
  const cache: ImageCache = new Map();

  const downloads = [...urls].map(async (url) => {
    const data = await downloadImage(url);
    cache.set(url, data);
  });

  await Promise.all(downloads);
  return cache;
}

export function convertImage(node: JSONContent, options: ConvertOptions = {}): Paragraph {
  const url = node.attrs?.url;
  const proportion = node.attrs?.proportion;
  const imageData = options.imageCache?.get(url);

  if (!url || !proportion || !imageData) {
    return createParagraph(
      {
        children: [new TextRun({ text: '[이미지]', italics: true })],
        alignment: AlignmentType.CENTER,
      },
      options,
    );
  }

  const { buffer: imageBuffer, width: imageWidth, height: imageHeight, type: imageType } = imageData;
  const aspectRatio = imageHeight / imageWidth;

  const pageLayout = options.pageLayout ?? createDefaultPageLayout('a4');

  const pageWidthMm = pageLayout.width;
  const marginLeftMm = pageLayout.marginLeft;
  const marginRightMm = pageLayout.marginRight;

  const availableWidthMm = pageWidthMm - marginLeftMm - marginRightMm;
  const availableWidthInches = mmToInch(availableWidthMm);

  const widthInches = availableWidthInches * proportion;
  const heightInches = widthInches * aspectRatio;

  const widthPixels = inchToPx(widthInches);
  const heightPixels = inchToPx(heightInches);

  const imageRun = new ImageRun({
    type: imageType,
    data: new Uint8Array(imageBuffer),
    transformation: {
      width: widthPixels,
      height: heightPixels,
    },
  });

  return createParagraph(
    {
      children: [imageRun],
      alignment: AlignmentType.CENTER,
      style: 'ImageCenter',
    },
    options,
  );
}
