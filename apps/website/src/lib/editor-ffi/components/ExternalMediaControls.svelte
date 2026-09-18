<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    meta: string;
    width: number;
    height: number;
    zoom: number;
    selected: boolean;
    pinned?: boolean;
    children: Snippet;
  };

  let { meta, width, height, zoom, selected, pinned = false, children }: Props = $props();

  const SCRIM_HEIGHT = 84;
  const BAR_GAP = 8;
  const MIN_WIDTH = 140;
  const MIN_HEIGHT = 44;

  const displayedWidth = $derived(width * zoom);
  const displayedHeight = $derived(height * zoom);
  const detached = $derived(displayedWidth < MIN_WIDTH || displayedHeight < MIN_HEIGHT);
  const shown = $derived(selected || pinned);
  const scrimHeight = $derived(Math.min(SCRIM_HEIGHT, displayedHeight) / zoom);
</script>

{#if detached}
  <div
    style:bottom={`calc(100% + ${BAR_GAP / zoom}px)`}
    style:opacity={shown ? '1' : undefined}
    class={css({
      position: 'absolute',
      left: '[50%]',
      transform: 'translateX(-50%)',
      zIndex: '10',
      opacity: '0',
      transition: 'common',
      pointerEvents: 'none',
      _groupActive: { opacity: '100', pointerEvents: 'auto' },
      _groupHover: { opacity: '100', pointerEvents: 'auto' },
      '&:focus-within': { opacity: '100', pointerEvents: 'auto' },
    })}
  >
    <div
      style:transform={zoom === 1 ? undefined : `scale(${1 / zoom})`}
      style:transform-origin="bottom center"
      class={flex({
        alignItems: 'center',
        gap: '8px',
        height: '36px',
        paddingLeft: '10px',
        paddingRight: '4px',
        borderRadius: '8px',
        backgroundColor: 'surface.inverse/80',
        backdropFilter: 'auto',
        backdropBlur: '8px',
      })}
    >
      {#if meta}
        <span class={css({ fontSize: '12px', color: 'text.on.inverse', whiteSpace: 'nowrap' })}>{meta}</span>
      {/if}
      {@render children()}
    </div>
  </div>
{:else}
  <div
    style:height={`${scrimHeight}px`}
    style:opacity={shown ? '1' : undefined}
    class={css({
      position: 'absolute',
      top: '0',
      left: '0',
      right: '0',
      borderTopRadius: '4px',
      backgroundImage:
        '[linear-gradient(to bottom, rgb(0 0 0 / 0.55) 0%, rgb(0 0 0 / 0.545) 10%, rgb(0 0 0 / 0.518) 20%, rgb(0 0 0 / 0.46) 30%, rgb(0 0 0 / 0.375) 40%, rgb(0 0 0 / 0.275) 50%, rgb(0 0 0 / 0.175) 60%, rgb(0 0 0 / 0.09) 70%, rgb(0 0 0 / 0.032) 80%, rgb(0 0 0 / 0.005) 90%, rgb(0 0 0 / 0) 100%)]',
      opacity: '0',
      transition: 'common',
      pointerEvents: 'none',
      overflow: 'hidden',
      _groupActive: { opacity: '100' },
      _groupHover: { opacity: '100' },
      '&:focus-within': { opacity: '100' },
    })}
  >
    <div
      style:width={`${100 * zoom}%`}
      style:transform={zoom === 1 ? undefined : `scale(${1 / zoom})`}
      style:transform-origin="top left"
      class={flex({
        alignItems: 'center',
        gap: '8px',
        height: '36px',
        paddingLeft: '12px',
        paddingRight: '6px',
        pointerEvents: 'none',
        _groupActive: { pointerEvents: 'auto' },
        _groupHover: { pointerEvents: 'auto' },
        '&:focus-within': { pointerEvents: 'auto' },
      })}
    >
      <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '12px', color: 'text.on.inverse', truncate: true })}>
        {meta}
      </span>
      {@render children()}
    </div>
  </div>
{/if}
