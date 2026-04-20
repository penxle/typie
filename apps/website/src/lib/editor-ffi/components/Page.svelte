<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { untrack } from 'svelte';
  import { getEditorContext } from '../editor.svelte';

  type Props = {
    page: number;
    width: number;
    height: number;
  };

  let { page, width, height }: Props = $props();

  const ctx = getEditorContext();
  const { editor } = ctx;

  const scaleFactor = $derived(ctx.editor?.scaleFactor ?? 1);
  const cssWidth = $derived(Math.round(width * scaleFactor) / scaleFactor);
  const cssHeight = $derived(Math.round(height * scaleFactor) / scaleFactor);
</script>

<div
  style:width={`${cssWidth}px`}
  style:height={`${cssHeight}px`}
  class={css({ position: 'relative', flexShrink: '0' })}
  {@attach (el) => {
    if (editor) {
      editor.pageEls[page] = el;

      return () => {
        editor.pageEls[page] = undefined;
      };
    }
  }}
>
  <canvas
    class={css({ height: 'full', width: 'full', imageRendering: 'pixelated' })}
    {@attach (canvas) => {
      if (!editor) return;

      untrack(() => {
        editor.attachSurface(page, canvas, width, height);
      });

      const off = editor.on('render_invalidated', () => {
        editor.renderSurface(page);
      });

      $effect.pre(() => {
        editor.resizeSurface(page, width, height);
        editor.renderSurface(page);
      });

      return () => {
        off();
        untrack(() => editor.detachSurface(page));
      };
    }}
  ></canvas>
</div>
