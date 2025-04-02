import init, { Glyphr } from '@typie/glyphr';
import * as Comlink from 'comlink';

let renderer: Glyphr;
let canvas: OffscreenCanvas;

const api = {
  init: async (_canvas: OffscreenCanvas) => {
    await init();
    canvas = _canvas;
  },

  loadFont: (fontData: Uint8Array) => {
    renderer = new Glyphr();
    renderer.load_font(fontData);
  },

  renderText: (text: string) => {
    if (!canvas) {
      return;
    }

    const ctx = canvas.getContext('2d');
    if (!ctx) {
      return;
    }

    ctx.clearRect(0, 0, canvas.width, canvas.height);

    let xPosition = 20;
    const yPosition = 100;

    // baseline
    ctx.strokeStyle = '#ccc';
    ctx.beginPath();
    ctx.moveTo(0, yPosition);
    ctx.lineTo(canvas.width, yPosition);
    ctx.stroke();

    for (let i = 0; i < text.length; i++) {
      try {
        // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
        const charCode = text.codePointAt(i)!;

        const metrics = renderer.get_metrics(charCode);

        const { buffer, width, height, top, left } = renderer.render_glyph(charCode);

        if (width === 0 || height === 0) {
          xPosition += metrics.advance_width;
          continue;
        }

        const imageData = ctx.createImageData(width, height);

        const data = imageData.data;

        for (let i = 0; i < width * height; i++) {
          const idx = i * 4;

          data[idx] = 0;
          data[idx + 1] = 0;
          data[idx + 2] = 0;
          data[idx + 3] = buffer[i];
        }

        const dx = Math.floor(xPosition + left);
        const dy = Math.floor(yPosition - top);

        ctx.putImageData(imageData, dx, dy);

        xPosition += metrics.advance_width;

        if (width > 0 && height > 0) {
          ctx.strokeStyle = 'rgba(0, 200, 0, 0.3)';
          ctx.strokeRect(dx, dy, width, height);
        }
      } catch (err) {
        console.error(err);
      }
    }
  },

  free: () => {
    renderer.free();
  },
};

export type WorkerApi = typeof api;

Comlink.expose(api);
