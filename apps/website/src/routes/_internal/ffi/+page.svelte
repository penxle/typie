<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { onMount } from 'svelte';
  import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
  import { handleFontDataMissing, handleFontManifestMissing, initFonts } from './fonts';
  import type { Doc, Editor, EditorEvent } from '@typie/editor-ffi/browser';

  const PRIMARY_FONT_PATHS: Record<string, string> = {
    Pretendard: 'Pretendard-Regular',
  };

  let editor = $state<Editor>();

  async function processEvents(ed: Editor, events: EditorEvent[]): Promise<void> {
    for (const event of events) {
      console.log(event);

      if (event.type === 'font_manifest_missing') {
        const { family, weight } = event.value;
        const fontPath = PRIMARY_FONT_PATHS[family];
        if (fontPath) {
          await handleFontManifestMissing(ed, family, weight, fontPath);
          await processEvents(ed, ed.tick());
        } else {
          console.warn(`Unknown primary font: ${family}`);
        }
      } else if (event.type === 'font_data_missing') {
        const { family, weight, required, prefetch } = event.value;
        await handleFontDataMissing(ed, family, weight, required, prefetch);
        await processEvents(ed, ed.tick());
      } else if (event.type === 'render_invalidated') {
        ed.render_surface(0);
      }
    }
  }

  const init = async () => {
    await initWasm();
    await initFonts();

    wasm.set_font_families([{ name: 'Pretendard', weights: [400] }]);

    const doc: Doc = {
      nodes: {
        '0': {
          node: { type: 'root' },
          modifiers: [
            { type: 'font_family', value: 'Pretendard' },
            { type: 'font_weight', value: 400 },
            { type: 'font_size', value: 1200 },
            { type: 'line_height', value: 160 },
            { type: 'letter_spacing', value: 0 },
            { type: 'text_color', value: 'black' },
            { type: 'paragraph_indent', value: 1 },
            { type: 'block_gap', value: 1 },
          ],
          children: ['1'],
        },
        '1': { node: { type: 'blockquote', variant: 'left_line' }, parent: '0', children: ['2'] },
        '2': { node: { type: 'paragraph' }, parent: '1', children: ['3'] },
        '3': { node: { type: 'text', text: '안녕하세요! Hello!' }, parent: '2' },
      },
      attrs: {
        layout_mode: {
          type: 'continuous',
          max_width: 400,
        },
      },
    };

    editor = wasm.create_editor(doc, { width: 400, height: 400, scale_factor: 2 });

    editor.enqueue({ type: 'system', value: { type: 'initialize' } });
    await processEvents(editor, editor.tick());
  };

  onMount(() => {
    init();
  });
</script>

<div class={center({ position: 'fixed', inset: '0', paddingX: '20px' })}>
  <canvas
    {@attach (el) => {
      el.style.width = '400px';
      el.style.height = '400px';
      el.style.imageRendering = 'pixelated';

      editor?.attach_surface(0, el, 400, 400, 2);
      editor?.render_surface(0);

      return () => {
        editor?.detach_surface(0);
      };
    }}
  ></canvas>
</div>
