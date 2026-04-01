<script lang="ts">
  import { center } from '@typie/styled-system/patterns';
  import { onMount } from 'svelte';
  import { initWasm, wasm } from '$lib/wasm-ffi.svelte';
  import { initFonts } from './fonts';
  import type { Editor } from '@typie/editor-ffi';

  let editor = $state<Editor>();

  const init = async () => {
    await initWasm();
    await initFonts();

    const doc = {
      nodes: {
        '0': {
          node: { type: 'root' },
          modifiers: [
            { type: 'font_family', value: 'Pretendard' },
            { type: 'font_weight', value: 400 },
          ],
          children: ['1'],
        },
        '1': { node: { type: 'paragraph' }, parent: '0', children: ['2'] },
        '2': { node: { type: 'text', text: 'hello' }, parent: '1' },
      },
      attrs: {
        layout_mode: {
          type: 'continuous',
          max_width: 600,
        },
      },
    };

    editor = wasm.create_editor(JSON.stringify(doc), { width: 100, height: 100, scale_factor: 2 });

    editor.enqueue({ System: 'Initialize' });
    const events = editor.tick();

    for (const event of events) {
      console.log(event);
    }
  };

  onMount(() => {
    init();
  });
</script>

<div class={center({ position: 'fixed', inset: '0', paddingX: '20px' })}>
  <canvas
    {@attach (el) => {
      console.log(editor);
      editor?.attach_surface(0, el, 100, 100, 2);
      editor?.resize_surface(0, 100, 100, 2);
      editor?.render_surface(0);

      return () => {
        editor?.detach_surface(0);
      };
    }}
  ></canvas>
</div>
