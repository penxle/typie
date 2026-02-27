<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getPaneGroup } from './context.svelte';
  import EntityPane from './EntityPane.svelte';
  import { computeLayout } from './geometry';
  import HomePane from './HomePane.svelte';
  import Resizer from './Resizer.svelte';
  import type { DropZone, Member } from './types';

  type Props = {
    root: Member;
  };

  let { root }: Props = $props();

  const context = getPaneGroup();

  let rootRef = $state<HTMLDivElement>();
  let rootWidth = $state(0);
  let rootHeight = $state(0);

  $effect(() => {
    if (!rootRef) return;

    const r = rootRef.getBoundingClientRect();
    rootWidth = r.width;
    rootHeight = r.height;

    const ro = new ResizeObserver(([e]) => {
      rootWidth = e.contentRect.width;
      rootHeight = e.contentRect.height;
    });
    ro.observe(rootRef);
    return () => ro.disconnect();
  });

  const layout = $derived.by(() => {
    if (rootWidth === 0 || rootHeight === 0) return null;
    return computeLayout(root, {
      left: 0,
      top: 0,
      width: rootWidth,
      height: rootHeight,
    });
  });

  $effect(() => {
    context.rootElement = rootRef ?? null;
  });

  $effect(() => {
    if (layout) {
      context.paneRects = layout.panes;
    }
  });

  const getOverlayRect = (paneRect: { left: number; top: number; width: number; height: number }, dropZone: DropZone) => {
    const inset = 20;
    switch (dropZone) {
      case 'center': {
        return {
          left: paneRect.left + inset,
          top: paneRect.top + inset,
          width: paneRect.width - inset * 2,
          height: paneRect.height - inset * 2,
        };
      }
      case 'left': {
        return { left: paneRect.left, top: paneRect.top, width: paneRect.width / 2, height: paneRect.height };
      }
      case 'right': {
        return { left: paneRect.left + paneRect.width / 2, top: paneRect.top, width: paneRect.width / 2, height: paneRect.height };
      }
      case 'top': {
        return { left: paneRect.left, top: paneRect.top, width: paneRect.width, height: paneRect.height / 2 };
      }
      case 'bottom': {
        return { left: paneRect.left, top: paneRect.top + paneRect.height / 2, width: paneRect.width, height: paneRect.height / 2 };
      }
    }
  };
</script>

<div
  bind:this={rootRef}
  class={css({
    position: 'relative',
    width: 'full',
    height: 'full',
    backgroundColor: 'surface.muted',
    overflow: 'hidden',
  })}
>
  {#each context.panes as pane (pane.id)}
    {@const rect = layout?.panes.get(pane.id)}
    {#if rect}
      <div
        style:position="absolute"
        style:left="{rect.left}px"
        style:top="{rect.top}px"
        style:width="{rect.width}px"
        style:height="{rect.height}px"
        style:overflow="hidden"
        style:display="grid"
      >
        {#key pane.kind === 'entity' ? pane.slug : pane.kind}
          <div style:grid-area="1/1" style:min-width="0" style:min-height="0">
            {#if pane.kind === 'entity'}
              <EntityPane {pane} />
            {:else if pane.kind === 'home'}
              <HomePane {pane} />
            {/if}
          </div>
        {/key}
      </div>
    {/if}
  {/each}

  {#if layout}
    {#each layout.resizers as resizer (resizer.id)}
      <Resizer {resizer} />
    {/each}
  {/if}

  {#if context.activeZone && layout}
    {@const paneRect = layout.panes.get(context.activeZone.paneId)}
    {#if paneRect}
      {@const overlayRect = getOverlayRect(paneRect, context.activeZone.dropZone)}
      <div
        style:position="absolute"
        style:left="{overlayRect.left}px"
        style:top="{overlayRect.top}px"
        style:width="{overlayRect.width}px"
        style:height="{overlayRect.height}px"
        style:pointer-events="none"
        style:transition="0.1s ease-in-out"
        class={css({
          backgroundColor: 'surface.dark',
          opacity: '[0.4]',
          borderRadius: '4px',
          zIndex: 'ghost',
        })}
      ></div>
    {/if}
  {/if}
</div>
