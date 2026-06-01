<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { zoomDiffers } from '$lib/editor-ffi/zoom';

  type Props = {
    isPaginated: boolean;
    displayZoom: number;
    scrollContainer?: HTMLElement;
  };

  const ZOOM_OVERLAY_VISIBLE_MS = 1000;
  const ZOOM_OVERLAY_FADE_MS = 180;

  let { isPaginated, displayZoom, scrollContainer }: Props = $props();

  let visible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let lastZoom: number | null = null;
  let wasPaginated = $state(false);
  let overlayLeft = $state(20);
  let overlayBottom = $state(20);

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
      if (hideTimer) {
        clearTimeout(hideTimer);
        hideTimer = null;
      }
    };
  });

  $effect(() => {
    const el = scrollContainer;

    const syncOverlayPosition = () => {
      if (!el) {
        overlayLeft = 20;
        overlayBottom = 20;
        return;
      }

      const rect = el.getBoundingClientRect();
      overlayLeft = rect.left + 20;
      overlayBottom = Math.max(20, window.innerHeight - rect.bottom + 20);
    };

    syncOverlayPosition();

    if (!el) {
      return;
    }

    const observer = new ResizeObserver(syncOverlayPosition);
    observer.observe(el);
    window.addEventListener('resize', syncOverlayPosition);

    return () => {
      observer.disconnect();
      window.removeEventListener('resize', syncOverlayPosition);
    };
  });

  const zoomPercent = $derived(Math.round(displayZoom * 100));
</script>

{#if isPaginated}
  <div
    style:left={`${overlayLeft}px`}
    style:bottom={`${overlayBottom}px`}
    style:opacity={visible ? '1' : '0'}
    style:transition={`opacity ${ZOOM_OVERLAY_FADE_MS}ms ease-out`}
    class={css({
      pointerEvents: 'none',
      position: 'fixed',
      zIndex: 'menu',
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
