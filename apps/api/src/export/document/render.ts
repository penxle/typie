import sharp from 'sharp';
import type { Editor, RenderInfo } from '@typie/editor';

export type RenderPageResult = {
  png: Uint8Array;
  width: number;
  height: number;
};

export async function renderDocumentPages(
  editor: Editor,
  getMemory: () => WebAssembly.Memory,
  pageCount: number,
): Promise<RenderPageResult[]> {
  const results: RenderPageResult[] = [];

  for (let i = 0; i < pageCount; i++) {
    const renderInfo: RenderInfo | undefined = editor.renderPage(i);
    if (!renderInfo) continue;

    const { ptr, len, width, height } = renderInfo;

    const wasmMemory = getMemory();
    const buffer = new Uint8Array(wasmMemory.buffer, ptr, len);
    const rgbaCopy = new Uint8Array(buffer);

    renderInfo.free();

    const pngBuffer = await sharp(rgbaCopy, {
      raw: {
        width,
        height,
        channels: 4,
      },
    })
      .png()
      .toBuffer();

    results.push({ png: new Uint8Array(pngBuffer), width, height });
  }

  return results;
}
