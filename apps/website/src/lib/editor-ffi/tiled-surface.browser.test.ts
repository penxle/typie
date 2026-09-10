import { expect, it, vi } from 'vitest';
import { initWasm } from '$lib/wasm-ffi.svelte';
import type { PlainDoc, PlainNodeEntry, Revision } from '@typie/editor-ffi/browser';

vi.mock('$env/dynamic/public', () => ({ env: {} }));
vi.mock('@mearie/svelte', async (importOriginal) => {
  const original = await importOriginal<typeof import('@mearie/svelte')>();
  return { ...original, createMutation: () => [vi.fn()] };
});

it('reuses canvas tiles and presents all accumulated damage only with the matching frame', async () => {
  const host = await initWasm();
  host.set_theme_variant('light-white')?.free();
  const plain: PlainDoc = {
    root: {
      node: { type: 'root', layout_mode: { type: 'continuous', max_width: 400 } },
      modifiers: {
        font_size: { type: 'font_size', value: 1600 },
        block_gap: { type: 'block_gap', value: 1000 },
      } as PlainNodeEntry['modifiers'],
      carry: [],
      children: ['first', 'middle', 'last'].map((text) => ({
        node: { type: 'callout' },
        modifiers: {} as PlainNodeEntry['modifiers'],
        carry: [],
        children: [
          {
            node: { type: 'paragraph' },
            modifiers: {} as PlainNodeEntry['modifiers'],
            carry: [],
            children: [{ node: { type: 'text', text }, modifiers: {} as PlainNodeEntry['modifiers'], carry: [], children: [] }],
          },
        ],
      })),
    },
  };
  const core = host.create_editor_from_doc(plain, { width: 400, height: 400, scale_factor: 2 });
  try {
    const surface = document.createElement('div');
    core.attach_surface(0, surface, 400, 400, 2);
    const prepare = (revision: Revision) => {
      const frame = core.render_surface(0, revision);
      if (!frame) throw new Error('Expected a prepared frame');
      return BigInt(frame.value);
    };
    const readPixels = (surface: HTMLElement) =>
      [...surface.querySelectorAll('canvas')].map((canvas) => {
        const context = canvas.getContext('2d');
        if (!context) throw new Error('Expected a canvas context');
        return context.getImageData(0, 0, canvas.width, canvas.height).data;
      });
    const samePixels = (a: Uint8ClampedArray[], b: Uint8ClampedArray[]) =>
      a.length === b.length &&
      a.every((pixels, index) => pixels.length === b[index].length && pixels.every((channel, offset) => channel === b[index][offset]));
    const initialized = core.tick_through(core.enqueue_request([{ type: 'system', event: { type: 'initialize' } }]));
    const firstFrame = prepare(initialized.revision);
    expect(core.present_surface(0, firstFrame)).toBe(true);
    const canvases = [...surface.querySelectorAll('canvas')];
    const before = readPixels(surface);
    expect(before.some((pixels) => pixels.some((channel) => channel !== 0))).toBe(true);
    let uploadedPixels = 0;
    const originalUpload = CanvasRenderingContext2D.prototype.putImageData;
    const upload = vi.spyOn(CanvasRenderingContext2D.prototype, 'putImageData').mockImplementation(function (
      this: CanvasRenderingContext2D,
      data,
      ...args
    ) {
      uploadedPixels += data.width * data.height;
      return Reflect.apply(originalUpload, this, [data, ...args]);
    });
    try {
      const change = (text: string) => {
        const selection = core.find_matches(text)[0];
        if (!selection) throw new Error('Expected a text range');
        core.tick_through(core.enqueue_request([{ type: 'selection', op: { type: 'set', selection } }]));
        const callout = core.block_state()?.ancestors.find((block) => block.node.type === 'callout');
        if (!callout) throw new Error('Expected a callout ancestor');
        const request = core.enqueue_request([{ type: 'node', op: { type: 'cycle_callout_variant', id: callout.id } }]);
        return core.tick_through(request).revision;
      };
      const intermediate = prepare(change('first'));
      const latestRevision = change('last');
      const latest = prepare(latestRevision);
      expect(core.present_surface(0, intermediate)).toBe(false);
      expect(uploadedPixels).toBe(0);
      expect(samePixels(readPixels(surface), before)).toBe(true);
      expect(core.present_surface(0, latest)).toBe(true);
      const displayed = [...surface.querySelectorAll('canvas')];
      expect(displayed).toHaveLength(canvases.length);
      for (const [index, canvas] of canvases.entries()) expect(displayed[index]).toBe(canvas);
      const incremental = readPixels(surface);
      expect(samePixels(incremental, before)).toBe(false);
      expect(uploadedPixels).toBeGreaterThan(0);
      expect(uploadedPixels).toBeLessThan(canvases.reduce((sum, canvas) => sum + canvas.width * canvas.height, 0) * 0.75);

      core.detach_surface(0);
      const fresh = document.createElement('div');
      core.attach_surface(0, fresh, 400, 400, 2);
      const freshFrame = prepare(latestRevision);
      expect(core.present_surface(0, freshFrame)).toBe(true);
      expect(samePixels(readPixels(fresh), incremental)).toBe(true);
      core.resize_surface(0, 402, 400, 2);
      expect(core.present_surface(0, freshFrame)).toBe(false);
      expect(core.present_surface(0, prepare(latestRevision))).toBe(true);
    } finally {
      upload.mockRestore();
    }
  } finally {
    core.free();
  }
});
