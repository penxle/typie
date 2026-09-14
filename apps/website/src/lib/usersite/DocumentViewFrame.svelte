<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { fade } from 'svelte/transition';
  import { documentViewColumnStyle, resolveDocumentViewColumn } from './document-view-layout';
  import DocumentViewSkeleton from './DocumentViewSkeleton.svelte';
  import type { Snippet } from 'svelte';
  import type { DocumentViewLayoutMode } from './document-view-layout';

  type Props = {
    layoutMode: DocumentViewLayoutMode;
    ready: boolean;
    header: Snippet;
    footer: Snippet;
    children: Snippet;
    bodySurface?: HTMLDivElement;
  };

  let { layoutMode, ready, header, footer, children, bodySurface = $bindable() }: Props = $props();

  const SWAP_DURATION_MS = 150;

  const column = $derived(resolveDocumentViewColumn(layoutMode));
  const columnStyle = $derived(documentViewColumnStyle(column));
  const background = $derived(token(column.paginated ? 'colors.surface.canvas' : 'colors.surface.default'));
  const swapDuration = $derived(prefersReducedMotion.current ? 0 : SWAP_DURATION_MS);
</script>

<div style:background-color={background} class={css({ display: 'flex', flexDirection: 'column' })}>
  <div style={columnStyle} class={css({ marginX: 'auto' })}>
    {@render header()}
  </div>

  <div class={css({ position: 'relative' }, !ready && { minHeight: '[100dvh]' })}>
    {#if !ready}
      <div
        style:background-color={background}
        class={css({ position: 'absolute', inset: '0', zIndex: 'editorOverlay' })}
        out:fade={{ duration: swapDuration }}
      >
        <div bind:this={bodySurface} style={columnStyle} class={css({ marginX: 'auto' })}>
          <DocumentViewSkeleton />
        </div>
      </div>
    {/if}

    <div
      class={css(
        { transition: '[opacity 150ms ease-out]', _motionReduce: { transition: '[none]' } },
        ready ? { opacity: '100' } : { position: 'absolute', top: '0', left: '0', right: '0', opacity: '0', pointerEvents: 'none' },
      )}
      inert={!ready}
    >
      {@render children()}
    </div>
  </div>

  <div style={columnStyle} class={css({ marginX: 'auto' })}>
    {@render footer()}
  </div>
</div>
