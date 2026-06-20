<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { zoomDiffers } from '$lib/editor/zoom';

  type Props = {
    isPaginated: boolean;
    displayZoom: number;
    useWindowScroll?: boolean;
  };

  const ZOOM_OVERLAY_VISIBLE_MS = 1000;
  const ZOOM_OVERLAY_FADE_MS = 180;

  let { isPaginated, displayZoom, useWindowScroll = false }: Props = $props();

  let visible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let lastZoom: number | null = null;
  let wasPaginated = $state(false);

  function showTemporarily() {
    visible = true;
    if (hideTimer) {
      clearTimeout(hideTimer);
    }
    hideTimer = setTimeout(() => {
      visible = false;
      hideTimer = null;
    }, ZOOM_OVERLAY_VISIBLE_MS);
  }

  $effect(() => {
    const enteredPaginated = isPaginated && !wasPaginated;
    wasPaginated = isPaginated;
    const prev = lastZoom;
    lastZoom = displayZoom;
    const shouldShow = prev === null || zoomDiffers(prev, displayZoom);
    if (isPaginated && (enteredPaginated || shouldShow)) {
      showTemporarily();
    }
    if (!isPaginated) {
      visible = false;
    }
  });

  $effect(() => {
    return () => {
      if (!hideTimer) {
        return;
      }

      clearTimeout(hideTimer);
      hideTimer = null;
    };
  });

  const zoomPercent = $derived(Math.round(displayZoom * 100));
</script>

{#if isPaginated}
  <div
    style:left="20px"
    style:bottom="20px"
    style:position={useWindowScroll ? 'fixed' : 'absolute'}
    style:opacity={visible ? '1' : '0'}
    style:transition={`opacity ${ZOOM_OVERLAY_FADE_MS}ms ease-out`}
    class={css({
      zIndex: 'menu',
      pointerEvents: 'none',
      borderRadius: '8px',
      borderWidth: '1px',
      borderColor: 'border.strong',
      backgroundColor: 'surface.subtle',
      opacity: '95',
      paddingX: '12px',
      paddingY: '8px',
      fontSize: '12px',
      fontWeight: 'medium',
      color: 'text.default',
      transition: 'opacity',
    })}
    aria-hidden={!visible}
    role="status"
  >
    {zoomPercent}%
  </div>
{/if}
