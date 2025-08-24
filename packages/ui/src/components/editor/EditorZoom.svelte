<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { onMount, tick } from 'svelte';
  import { browser } from '$app/environment';
  import { clamp, mmToPx } from '../../utils';
  import type { Snippet } from 'svelte';
  import type { LayoutMode, PageLayout } from '../../utils/page-layout';

  type Props = {
    layoutMode: LayoutMode;
    pageLayout?: PageLayout;
    scrollContainer?: HTMLDivElement;
    class?: string;
    scale?: number;
    zoomed?: boolean;
    children: Snippet;
  };

  let {
    layoutMode,
    pageLayout,
    scrollContainer,
    class: className,
    scale = $bindable(1),
    zoomed = $bindable(false),
    children,
  }: Props = $props();

  let isPinching = $state(false);
  let lastPinchDistance = $state(0);
  let userScale = $state(1);
  let zoomOrigin = $state<{ x: number; y: number; scrollX: number; scrollY: number; scale: number } | null>(null);

  const baseScale = $derived(() => {
    if (!browser) return 1;
    if (!(layoutMode === 'page' && pageLayout)) return 1;
    const screenWidth = window.innerWidth;
    const pageWidthPx = mmToPx(pageLayout.width);
    return Math.min(1, screenWidth / pageWidthPx);
  });

  const editorScale = $derived(() => {
    const scale = baseScale() * userScale;
    return Math.abs(scale - 1) < 0.0001 ? 1 : scale;
  });

  $effect(() => {
    scale = editorScale();
    zoomed = editorScale() > baseScale();
  });

  const updateScrollForZoom = async (
    scrollContainer: HTMLDivElement,
    originScale: number,
    newScale: number,
    originX: number,
    originY: number,
    originScrollX: number,
    originScrollY: number,
  ) => {
    const clientWidth = scrollContainer.clientWidth;
    const clientHeight = scrollContainer.clientHeight;

    const scaleRatio = newScale / originScale;
    const absoluteX = originScrollX + originX;
    const absoluteY = originScrollY + originY;

    await tick();

    const newScrollLeft = absoluteX * scaleRatio - originX;
    const newScrollTop = absoluteY * scaleRatio - originY;

    const newScrollWidth = scrollContainer.scrollWidth;
    const newScrollHeight = scrollContainer.scrollHeight;

    scrollContainer.scrollLeft = clamp(newScrollLeft, 0, newScrollWidth - clientWidth);
    scrollContainer.scrollTop = clamp(newScrollTop, 0, newScrollHeight - clientHeight);
  };

  const handleTouchStart = (e: TouchEvent) => {
    if (e.touches.length === 2) {
      e.preventDefault();
      isPinching = true;
      const touch1 = e.touches[0];
      const touch2 = e.touches[1];
      lastPinchDistance = Math.hypot(touch2.clientX - touch1.clientX, touch2.clientY - touch1.clientY);
    }
  };

  const handleTouchMove = async (e: TouchEvent) => {
    if (!isPinching || e.touches.length !== 2) return;
    e.preventDefault();
    if (!(layoutMode === 'page' && pageLayout)) return;
    if (!scrollContainer) return;

    const touch1 = e.touches[0];
    const touch2 = e.touches[1];
    const currentDistance = Math.hypot(touch2.clientX - touch1.clientX, touch2.clientY - touch1.clientY);
    const prevScale = editorScale();
    const rect = scrollContainer.getBoundingClientRect();
    const centerX = (touch1.clientX + touch2.clientX) / 2 - rect.left;
    const centerY = (touch1.clientY + touch2.clientY) / 2 - rect.top;
    const delta = currentDistance - lastPinchDistance;

    if (!zoomOrigin) {
      zoomOrigin = {
        x: centerX,
        y: centerY,
        scrollX: scrollContainer.scrollLeft,
        scrollY: scrollContainer.scrollTop,
        scale: prevScale,
      };
    }

    const scaleDelta = delta * 0.01;
    const newUserScale = userScale + scaleDelta;
    const currentBaseScale = baseScale();
    const clampedUserScale = clamp(newUserScale, 1, 1 / currentBaseScale);

    if (currentBaseScale <= 1 && clampedUserScale !== userScale) {
      userScale = clampedUserScale;
      const newScale = clampedUserScale * currentBaseScale;

      if (!scrollContainer) return;
      await updateScrollForZoom(
        scrollContainer,
        zoomOrigin.scale,
        newScale,
        zoomOrigin.x,
        zoomOrigin.y,
        zoomOrigin.scrollX,
        zoomOrigin.scrollY,
      );
    }

    lastPinchDistance = currentDistance;
  };

  const handleTouchEnd = (e: TouchEvent) => {
    if (e.touches.length < 2) {
      isPinching = false;
      zoomOrigin = null;
    }
  };

  let wheelTimer: ReturnType<typeof setTimeout> | null = null;

  const handleWheel = async (e: WheelEvent) => {
    if (!e.ctrlKey && !e.metaKey) return;
    e.preventDefault();

    if (!(layoutMode === 'page' && pageLayout)) return;
    if (!scrollContainer) return;

    const prevScale = editorScale();
    const rect = scrollContainer.getBoundingClientRect();
    const localX = e.clientX - rect.left;
    const localY = e.clientY - rect.top;

    if (!zoomOrigin) {
      zoomOrigin = {
        x: localX,
        y: localY,
        scrollX: scrollContainer.scrollLeft,
        scrollY: scrollContainer.scrollTop,
        scale: prevScale,
      };
    }

    const scaleDelta = -e.deltaY * 0.01;
    const newUserScale = userScale + scaleDelta;
    const currentBaseScale = baseScale();
    const clampedUserScale = clamp(newUserScale, 1, 1 / currentBaseScale);

    if (currentBaseScale <= 1 && clampedUserScale !== userScale) {
      userScale = clampedUserScale;
      const newScale = clampedUserScale * currentBaseScale;

      if (!scrollContainer) return;
      await updateScrollForZoom(
        scrollContainer,
        zoomOrigin.scale,
        newScale,
        zoomOrigin.x,
        zoomOrigin.y,
        zoomOrigin.scrollX,
        zoomOrigin.scrollY,
      );
    }

    if (wheelTimer) clearTimeout(wheelTimer);
    wheelTimer = setTimeout(() => {
      zoomOrigin = null;
      wheelTimer = null;
    }, 150);
  };

  const alignSelf = $derived(editorScale() > baseScale() ? 'flex-start' : 'center');

  let containerRef = $state<HTMLDivElement>();

  onMount(() => {
    if (!containerRef) return;

    containerRef.addEventListener('touchstart', handleTouchStart, { passive: false });
    containerRef.addEventListener('touchmove', handleTouchMove, { passive: false });
    containerRef.addEventListener('touchend', handleTouchEnd, { passive: false });

    return () => {
      if (!containerRef) return;
      containerRef.removeEventListener('touchstart', handleTouchStart);
      containerRef.removeEventListener('touchmove', handleTouchMove);
      containerRef.removeEventListener('touchend', handleTouchEnd);
    };
  });
</script>

{#if layoutMode === 'page'}
  <div
    bind:this={containerRef}
    style:align-self={alignSelf}
    style:width={editorScale() > baseScale() ? `calc(var(--prosemirror-max-width) * ${editorScale()})` : '100%'}
    class={cx(
      className,
      flex({
        height: '[inherit]',
        direction: 'column',
        alignItems: 'center',
        touchAction: 'auto',
      }),
    )}
    onwheel={handleWheel}
  >
    <div
      style:transform={`scale(${editorScale()})`}
      style:transform-origin="center top"
      style:will-change={editorScale() === 1 ? 'auto' : 'transform'}
      class={css({
        width: 'full',
      })}
    >
      {@render children()}
    </div>
  </div>

  {#if editorScale() < 1}
    <div
      class={css({
        position: 'fixed',
        left: '20px',
        bottom: '20px',
        paddingX: '12px',
        paddingY: '8px',
        backgroundColor: 'surface.subtle',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '8px',
        fontSize: '12px',
        color: 'text.subtle',
      })}
    >
      {Math.round(editorScale() * 100)}%
    </div>
  {/if}
{:else}
  <div class={cx(className, css({ width: 'full' }))}>
    {@render children()}
  </div>
{/if}
