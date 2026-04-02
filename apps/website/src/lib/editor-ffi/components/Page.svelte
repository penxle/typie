<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditorContext } from '../editor.svelte';

  type Props = {
    page: number;
    width: number;
    height: number;
  };

  let { page, width, height }: Props = $props();

  const { editor } = getEditorContext();
</script>

<div
  style:width={`${width}px`}
  style:height={`${height}px`}
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
      if (editor) {
        editor.attachSurface(page, canvas, width, height);

        const off = editor.on('render_invalidated', () => {
          editor.renderSurface(page);
        });

        $effect(() => {
          editor.resizeSurface(page, width, height);
        });

        return () => {
          off();
          editor.detachSurface(page);
        };
      }
    }}
  ></canvas>
</div>
