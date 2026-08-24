<script lang="ts">
  import { presentedPageElement } from '$lib/editor-ffi/geometry';
  import type { PageRect } from '@typie/editor-ffi/browser';
  import type { Editor } from '$lib/editor-ffi/editor.svelte';

  type Props = {
    editor: Editor;
    edge: string;
    fill: string;
    kind: 'issue' | 'strength' | null;
    rect: PageRect;
  };

  let { editor, edge, fill, kind, rect }: Props = $props();

  const container = $derived(presentedPageElement(editor, rect.page_idx));
  let element = $state<HTMLDivElement>();

  $effect(() => {
    if (container && element && element.parentElement !== container) container.append(element);
  });
</script>

<div
  bind:this={element}
  style:--review-edge={edge}
  style:--review-fill={fill}
  style:left={`${rect.rect.x}px`}
  style:top={`${rect.rect.y}px`}
  style:width={`${rect.rect.width}px`}
  style:height={`${rect.rect.height}px`}
  class="prism-review-highlight"
  data-prism-review-highlight={kind}
></div>

<style>
  .prism-review-highlight {
    position: absolute;
    overflow: hidden;
    border-radius: 3px;
    pointer-events: none;
  }

  .prism-review-highlight::before,
  .prism-review-highlight::after {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    content: '';
    pointer-events: none;
  }

  .prism-review-highlight::before {
    color: var(--review-fill);
    background: linear-gradient(
      180deg,
      color-mix(in oklch, currentColor 4%, transparent) 0%,
      color-mix(in oklch, currentColor 4%, transparent) 52%,
      color-mix(in oklch, currentColor 15%, transparent) 74%,
      color-mix(in oklch, currentColor 36%, transparent) 100%
    );
    mix-blend-mode: color;
  }

  .prism-review-highlight::after {
    color: var(--review-edge);
    box-shadow: inset 0 -1px color-mix(in oklch, var(--review-edge) 72%, transparent);
  }
</style>
