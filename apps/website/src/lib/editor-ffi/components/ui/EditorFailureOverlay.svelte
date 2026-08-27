<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Grain } from '@typie/ui/effects';
  import { fade, scale } from 'svelte/transition';
  import Logo from '$assets/logos/logo.svg?component';

  type Rectangle = {
    top: number;
    left: number;
    width: number;
    height: number;
  };

  type Props = {
    id: string;
    actionLabel: string;
    onAction: () => void;
  } & (
    | { contentPosition?: 'surface'; surfaceElement?: never; minimumWidth?: never }
    | { contentPosition: 'viewport'; surfaceElement: HTMLElement; minimumWidth?: number }
  );

  let { id, actionLabel, onAction, contentPosition = 'surface', surfaceElement, minimumWidth }: Props = $props();

  let overlayElement: HTMLElement | undefined = $state();
  let actionButton: HTMLElement | undefined = $state();
  let measuredSurface: { element: HTMLElement; bounds: Rectangle } | undefined = $state();
  let hasFocusedAction = false;
  const titleId = $derived(`editor-failure-title-${id}`);
  const messageId = $derived(`editor-failure-message-${id}`);
  const visibleBounds = $derived.by(() => {
    const measured = measuredSurface;
    return contentPosition === 'viewport' && measured !== undefined && measured.element === surfaceElement ? measured.bounds : undefined;
  });
  const overlayVisible = $derived(contentPosition === 'surface' || visibleBounds !== undefined);

  function measureSurfaceBounds(element: HTMLElement, overlay: HTMLElement): Rectangle | undefined {
    const containingBlock = overlay.offsetParent;
    if (!(containingBlock instanceof HTMLElement)) return undefined;

    const surface = element.getBoundingClientRect();
    const container = containingBlock.getBoundingClientRect();
    return {
      top: surface.top - container.top - containingBlock.clientTop + containingBlock.scrollTop,
      left: surface.left - container.left - containingBlock.clientLeft + containingBlock.scrollLeft,
      width: surface.width,
      height: surface.height,
    };
  }

  $effect(() => {
    const element = contentPosition === 'viewport' ? surfaceElement : undefined;
    const overlay = overlayElement;
    if (!element || !overlay) {
      measuredSurface = undefined;
      return;
    }

    const sync = () => {
      const bounds = measureSurfaceBounds(element, overlay);
      measuredSurface = bounds ? { element, bounds } : undefined;
    };
    const resizeObserver = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(sync);
    const containingBlock = overlay.offsetParent;

    sync();
    window.addEventListener('resize', sync);
    resizeObserver?.observe(element);
    if (containingBlock instanceof HTMLElement) resizeObserver?.observe(containingBlock);

    return () => {
      window.removeEventListener('resize', sync);
      resizeObserver?.disconnect();
    };
  });

  $effect(() => {
    if (hasFocusedAction || !actionButton || (contentPosition === 'viewport' && !visibleBounds)) return;
    hasFocusedAction = true;
    actionButton.focus({ preventScroll: true });
  });
</script>

<div
  bind:this={overlayElement}
  style:top={visibleBounds ? `${visibleBounds.top}px` : undefined}
  style:left={visibleBounds ? `${visibleBounds.left}px` : undefined}
  style:width={visibleBounds ? `${visibleBounds.width}px` : undefined}
  style:height={visibleBounds ? `${visibleBounds.height}px` : undefined}
  style:min-width={minimumWidth ? `${minimumWidth}px` : undefined}
  style:visibility={overlayVisible ? undefined : 'hidden'}
  style:pointer-events={overlayVisible ? undefined : 'none'}
  class={css({
    position: 'absolute',
    inset: contentPosition === 'surface' ? '0' : undefined,
    zIndex: 'editorOverlay',
    overflow: contentPosition === 'surface' ? 'hidden' : 'clip',
  })}
  aria-describedby={messageId}
  aria-labelledby={titleId}
  role="alertdialog"
>
  {#if overlayVisible}
    <div class={css({ position: 'absolute', inset: '0' })} in:fade|global={{ duration: 150 }}>
      <Grain style={css.raw({ position: 'absolute', inset: '0' })} freq={2.2} opacity={0.5} />

      <div
        class={flex({
          position: contentPosition === 'surface' ? 'absolute' : 'sticky',
          inset: contentPosition === 'surface' ? '0' : undefined,
          top: contentPosition === 'viewport' ? '0' : undefined,
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'flex-start',
          width: contentPosition === 'viewport' ? 'full' : undefined,
          height: contentPosition === 'viewport' ? '[min(100dvh, 100%)]' : undefined,
          boxSizing: 'border-box',
          padding: '20px',
          overflowY: 'auto',
          pointerEvents: 'auto',
        })}
      >
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            gap: '14px',
            borderRadius: '12px',
            padding: { base: '24px', lg: '32px' },
            boxSizing: 'border-box',
            width: 'full',
            minWidth: '0',
            maxWidth: '340px',
            flexShrink: '0',
            marginY: 'auto',
            backgroundColor: 'surface.default',
            textAlign: 'center',
            boxShadow: 'small',
            zIndex: '1',
            pointerEvents: 'auto',
          })}
          in:scale|global={{ start: 0.96, duration: 150 }}
        >
          <Logo class={css({ width: '24px', height: '24px' })} />
          <h2 id={titleId} class={css({ fontSize: '20px', fontWeight: 'bold' })}>앗! 문제가 발생했어요</h2>
          <p id={messageId} class={css({ fontSize: '14px', color: 'text.faint' })}>잠시 후 다시 시도해주세요.</p>
          <Button onclick={onAction} size="md" bind:element={actionButton}>{actionLabel}</Button>
        </div>
      </div>
    </div>
  {/if}
</div>
