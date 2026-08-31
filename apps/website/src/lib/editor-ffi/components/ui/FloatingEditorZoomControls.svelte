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
    fixed?: boolean;
    revealOnHover?: boolean;
    rightInset?: number;
  };

  const CONTINUOUS_SURFACE = token('colors.surface.default');
  const PAGINATED_SURFACE = token('colors.surface.subtle');
  const TRANSIENT_BASE = `color-mix(in srgb, ${CONTINUOUS_SURFACE} 50%, ${PAGINATED_SURFACE} 50%)`;
  const TRANSIENT_SURFACE = `color-mix(in srgb, ${TRANSIENT_BASE} 85%, transparent)`;

  let { controls, fixed = false, revealOnHover = false, rightInset = 0 }: Props = $props();
  const visibility = new TransientVisibilityState();
  const visible = $derived(controls.enabled && visibility.visible);
  const engaged = $derived(visibility.engaged);

  function handlePointerEnter(): void {
    if (controls.enabled && (revealOnHover || visible)) visibility.setHovered(true);
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
  style:position={fixed ? 'fixed' : 'absolute'}
  style:top={fixed ? 'var(--usersite-sticky-header-bottom, 0px)' : '0'}
  style:right={`${rightInset}px`}
  class={css({
    zIndex: '5',
    display: 'flex',
    justifyContent: 'flex-end',
    pointerEvents: 'none',
  })}
  data-floating-editor-zoom-anchor
  data-floating-editor-zoom-fixed={fixed}
  data-floating-editor-zoom-right-inset={rightInset}
  role="presentation"
>
  <div
    style:color={token(engaged ? 'colors.text.default' : 'colors.text.subtle')}
    style:opacity={visible ? '1' : '0'}
    style:pointer-events={revealOnHover || visible ? 'auto' : 'none'}
    style:transition={prefersReducedMotion.current
      ? 'none'
      : `opacity ${visible ? CONTEXT_BAR_FADE_IN_MS : CONTEXT_BAR_FADE_OUT_MS}ms ease-out, color ${CONTEXT_BAR_FADE_IN_MS}ms ease-out`}
    class={css({ position: 'relative', isolation: 'isolate', width: '[fit-content]', borderRadius: '8px' })}
    data-floating-editor-zoom-controls
    data-floating-editor-zoom-hover-reveal={revealOnHover}
    onfocusin={handleFocusIn}
    onfocusout={handleFocusOut}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    role="presentation"
  >
    <div
      style:backdrop-filter={`blur(${visible ? 2 : 0}px)`}
      style:transition={prefersReducedMotion.current ? 'none' : 'backdrop-filter 320ms cubic-bezier(0, 0, 0.2, 1)'}
      class={css({ position: 'absolute', inset: '0', zIndex: '[-2]', borderRadius: '8px', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-blur
    ></div>
    <div
      style:background-color={TRANSIENT_SURFACE}
      class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', borderRadius: '8px', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-surface
    ></div>
    <EditorZoomControls {...controls} keyboardDiscoverableWhenHidden {visibility} {visible} />
  </div>
</div>
