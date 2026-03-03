<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getAppContext } from '@typie/ui/context';
  import { fade } from 'svelte/transition';
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

  const app = getAppContext();
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
    const inset = 6;
    const gap = 3;
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
        return {
          left: paneRect.left + inset,
          top: paneRect.top + inset,
          width: paneRect.width / 2 - inset - gap,
          height: paneRect.height - inset * 2,
        };
      }
      case 'right': {
        return {
          left: paneRect.left + paneRect.width / 2 + gap,
          top: paneRect.top + inset,
          width: paneRect.width / 2 - inset - gap,
          height: paneRect.height - inset * 2,
        };
      }
      case 'top': {
        return {
          left: paneRect.left + inset,
          top: paneRect.top + inset,
          width: paneRect.width - inset * 2,
          height: paneRect.height / 2 - inset - gap,
        };
      }
      case 'bottom': {
        return {
          left: paneRect.left + inset,
          top: paneRect.top + paneRect.height / 2 + gap,
          width: paneRect.width - inset * 2,
          height: paneRect.height / 2 - inset - gap,
        };
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
        style:left="{rect.left}px"
        style:top="{rect.top}px"
        style:width="{rect.width}px"
        style:height="{rect.height}px"
        style:opacity={context.draggingPaneId === pane.id ? '0.4' : undefined}
        style:scale={context.draggingPaneId === pane.id ? '0.99' : undefined}
        style:z-index={app.preference.current.zenModeEnabled && context.state.current.focusedPaneId === pane.id ? '70' : undefined}
        class={css({
          position: 'absolute',
          overflow: 'hidden',
          display: 'grid',
          isolation: 'isolate',
          transition: '[opacity 150ms ease-out, scale 150ms ease-out]',
        })}
      >
        <div class={css({ gridArea: '[1/1]', minWidth: '0', minHeight: '0' })}>
          {#if pane.kind === 'entity'}
            <EntityPane {pane} />
          {:else if pane.kind === 'home'}
            <HomePane {pane} />
          {/if}
        </div>
      </div>
    {/if}
  {/each}

  {#if layout}
    {#each layout.resizers as resizer (resizer.id)}
      <Resizer {resizer} />
    {/each}
  {/if}

  {#if context.enabled && !context.draggingPaneId && !app.preference.current.zenModeEnabled && layout}
    {@const focusedRect = layout.panes.get(context.state.current.focusedPaneId ?? '')}
    {#if focusedRect}
      <div
        style:left="{focusedRect.left}px"
        style:top="{focusedRect.top}px"
        style:width="{focusedRect.width}px"
        style:height="{focusedRect.height}px"
        class={css({
          position: 'absolute',
          pointerEvents: 'none',
          boxShadow: '[0 0 0 1px token(colors.border.default)]',
          zIndex: 'overEditor',
        })}
        transition:fade|global={{ duration: 150 }}
      ></div>
    {/if}
  {/if}

  {#if context.draggingPaneId}
    <div class={css({ position: 'absolute', inset: '0', zIndex: 'ghost' })}></div>
  {/if}

  {#if context.activeZone && layout}
    {@const paneRect = layout.panes.get(context.activeZone.paneId)}
    {#if paneRect}
      {@const overlayRect = getOverlayRect(paneRect, context.activeZone.dropZone)}
      <div
        style:left="{overlayRect.left}px"
        style:top="{overlayRect.top}px"
        style:width="{overlayRect.width}px"
        style:height="{overlayRect.height}px"
        class={css({
          position: 'absolute',
          pointerEvents: 'none',
          transition:
            '[left 150ms cubic-bezier(0.2,0,0,1), top 150ms cubic-bezier(0.2,0,0,1), width 150ms cubic-bezier(0.2,0,0,1), height 150ms cubic-bezier(0.2,0,0,1)]',
          backgroundColor: { base: 'accent.info.default/8', _dark: 'accent.info.default/15' },
          borderWidth: '[1.5px]',
          borderStyle: 'solid',
          borderColor: { base: 'accent.info.default/30', _dark: 'accent.info.default/40' },
          borderRadius: '8px',
          zIndex: 'ghost',
        })}
        in:fade|global={{ duration: 150 }}
      ></div>
    {/if}
  {/if}
</div>
