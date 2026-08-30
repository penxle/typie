<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { CONTEXT_BAR_FADE_IN_MS, CONTEXT_BAR_FADE_OUT_MS, CONTEXT_BAR_TRANSIENT_VISIBLE_MS } from './editor-context-bar.svelte';
  import EditorZoomControls from './EditorZoomControls.svelte';
  import { TransientVisibilityState } from './transient-visibility.svelte';
  import type { EditorZoomControlsRenderProps } from './EditorZoomControls.svelte';

  type Props = {
    controls: EditorZoomControlsRenderProps;
  };

  const FADE_WIDTH = 24;
  const ACTIVE_SURFACE = token('colors.surface.muted');
  const CONTINUOUS_SURFACE = token('colors.surface.default');
  const PAGINATED_SURFACE = token('colors.surface.subtle');
  const TRANSIENT_BASE = `color-mix(in srgb, ${CONTINUOUS_SURFACE} 50%, ${PAGINATED_SURFACE} 50%)`;
  const SUBTLE_BORDER = token('colors.border.subtle');
  const DEFAULT_BORDER = token('colors.border.default');
  const TRANSIENT_SURFACE = `color-mix(in srgb, ${TRANSIENT_BASE} 85%, transparent)`;
  const TRANSIENT_EDGE = `color-mix(in srgb, ${TRANSIENT_BASE} 40%, ${SUBTLE_BORDER} 60%)`;
  const ENGAGED_EDGE = `color-mix(in srgb, ${ACTIVE_SURFACE} 36%, ${DEFAULT_BORDER} 64%)`;

  let { controls }: Props = $props();
  const visibility = new TransientVisibilityState();
  const visible = $derived(controls.enabled && visibility.visible);
  const engaged = $derived(visibility.engaged);
  const surfaceColor = $derived(engaged ? ACTIVE_SURFACE : TRANSIENT_SURFACE);
  const edgeColor = $derived(engaged ? ENGAGED_EDGE : TRANSIENT_EDGE);

  const smootherstep = (value: number): number => {
    const x = Math.min(1, Math.max(0, value));
    return x * x * x * (x * (x * 6 - 15) + 10);
  };
  const fadeStops = Array.from({ length: 9 }, (_, index) => {
    const progress = index / 8;
    return `rgb(0 0 0 / ${smootherstep(progress)}) ${FADE_WIDTH * progress}px`;
  });
  const fadeMask = `linear-gradient(to right, ${fadeStops.join(', ')}, black ${FADE_WIDTH}px, black 100%)`;

  function handlePointerEnter(): void {
    if (visible) visibility.setHovered(true);
  }

  function handlePointerLeave(): void {
    if (visibility.hovered) visibility.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    visibility.setHovered(false);
  }

  function handleFocusIn(): void {
    visibility.setFocused(true);
  }

  function handleFocusOut(event: FocusEvent & { currentTarget: HTMLDivElement }): void {
    const nextInside = event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget);
    if (!nextInside && visibility.focused) visibility.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    visibility.setFocused(nextInside);
  }

  $effect(() => {
    if (!controls.enabled) visibility.destroy();
  });

  $effect(() => () => visibility.destroy());
</script>

<div
  class={css({
    position: 'fixed',
    top: '[var(--usersite-sticky-header-bottom, 0px)]',
    right: '0',
    zIndex: '5',
    display: 'flex',
    justifyContent: 'flex-end',
    height: '0',
    pointerEvents: 'none',
  })}
  data-floating-editor-zoom-anchor
  role="presentation"
>
  <div
    style:color={token(engaged ? 'colors.text.default' : 'colors.text.subtle')}
    style:opacity={visible ? '1' : '0'}
    style:pointer-events={visible ? 'auto' : 'none'}
    style:transition={prefersReducedMotion.current
      ? 'none'
      : `opacity ${visible ? CONTEXT_BAR_FADE_IN_MS : CONTEXT_BAR_FADE_OUT_MS}ms ease-out, color ${CONTEXT_BAR_FADE_IN_MS}ms ease-out`}
    class={css({ position: 'relative', isolation: 'isolate', width: '[fit-content]' })}
    data-floating-editor-zoom-controls
    onfocusin={handleFocusIn}
    onfocusout={handleFocusOut}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    role="presentation"
  >
    <div
      style:backdrop-filter={`blur(${visible && !engaged ? 2 : 0}px)`}
      style:mask-image={fadeMask}
      style:transition={prefersReducedMotion.current ? 'none' : 'backdrop-filter 320ms cubic-bezier(0, 0, 0.2, 1)'}
      class={css({ position: 'absolute', top: '0', right: '0', bottom: '0', left: '-24px', zIndex: '[-2]', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-blur
    ></div>
    <div
      style:background-color={surfaceColor}
      style:mask-image={fadeMask}
      style:transition={prefersReducedMotion.current ? 'none' : `background-color ${CONTEXT_BAR_FADE_IN_MS}ms ease-out`}
      class={css({ position: 'absolute', top: '0', right: '0', bottom: '0', left: '-24px', zIndex: '[-1]', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-surface
    ></div>
    <div
      style:background-color={edgeColor}
      style:mask-image={fadeMask}
      style:transition={prefersReducedMotion.current ? 'none' : `background-color ${CONTEXT_BAR_FADE_IN_MS}ms ease-out`}
      class={css({ position: 'absolute', right: '0', bottom: '0', left: '-24px', zIndex: '[-1]', height: '1px', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-edge
    ></div>

    <EditorZoomControls {...controls} keyboardDiscoverableWhenHidden {visibility} {visible} />
  </div>
</div>
