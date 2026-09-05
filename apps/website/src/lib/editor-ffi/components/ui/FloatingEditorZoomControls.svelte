<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { prefersReducedMotion } from '@typie/ui/state';
  import EditorZoomControls from './EditorZoomControls.svelte';
  import { setupFloatingOverlayPosition } from './floating-overlay-position.svelte';
  import type { EditorZoomControlsRenderProps } from './EditorZoomControls.svelte';

  export type FloatingEditorZoomChromeAttachment = {
    hold(event?: PointerEvent): void;
    release(): void;
    discoverable(): boolean;
    attached(): boolean;
    pointer(): { x: number; y: number } | null;
  };

  type Props = {
    controls: EditorZoomControlsRenderProps;
    fixed?: boolean;
    revealOnHover?: boolean;
    rightInset?: number;
    topInset?: number;
    layoutOriginOffset?: number;
    chromeAttachment?: FloatingEditorZoomChromeAttachment;
  };

  const CONTINUOUS_SURFACE = token('colors.surface.default');
  const PAGINATED_SURFACE = token('colors.surface.canvas');
  const TRANSIENT_BASE = `color-mix(in srgb, ${CONTINUOUS_SURFACE} 50%, ${PAGINATED_SURFACE} 50%)`;
  const TRANSIENT_SURFACE = `color-mix(in srgb, ${TRANSIENT_BASE} 75%, transparent)`;
  const TRANSIENT_VISIBLE_MS = 1500;
  const FADE_IN_MS = 180;
  const FADE_OUT_MS = 400;

  let {
    controls,
    fixed = false,
    revealOnHover = false,
    rightInset = 0,
    topInset,
    layoutOriginOffset = 0,
    chromeAttachment,
  }: Props = $props();
  let transient = $state(false);
  let hovered = $state(false);
  let focused = $state(false);
  let feedbackVisibilityHeld = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  const visible = $derived(controls.enabled && (transient || hovered || focused || feedbackVisibilityHeld));
  const chromeDiscoverable = $derived(chromeAttachment?.discoverable() ?? true);
  const presented = $derived(visible);
  const engaged = $derived(hovered || focused);
  const attached = $derived(chromeAttachment?.attached() ?? false);
  const interactive = $derived(presented || (chromeDiscoverable && revealOnHover));
  const top = $derived(
    fixed
      ? 'var(--usersite-sticky-header-bottom, 0px)'
      : topInset === undefined
        ? 'var(--editor-floating-zoom-top-inset, 0px)'
        : `${topInset}px`,
  );
  const borderRadius = $derived(attached ? '0 0 8px 8px' : '8px');
  let anchor: HTMLDivElement;
  let controlsElement: HTMLDivElement;
  let pointerSyncFrame: number | undefined;

  function showTemporarily(additionalDurationMs = 0): void {
    transient = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(() => {
      transient = false;
      hideTimer = undefined;
    }, TRANSIENT_VISIBLE_MS + additionalDurationMs);
  }

  function clearVisibility(): void {
    clearTimeout(hideTimer);
    hideTimer = undefined;
    transient = false;
    hovered = false;
    focused = false;
    feedbackVisibilityHeld = false;
  }

  setupFloatingOverlayPosition({
    element: () => anchor,
    layoutTop: () => (fixed || topInset === undefined ? undefined : layoutOriginOffset + topInset),
    presented: () => presented,
  });

  function syncPointer(): void {
    pointerSyncFrame = undefined;
    const pointer = chromeAttachment?.pointer();
    if (!controlsElement || !pointer) return;
    const rect = controlsElement.getBoundingClientRect();
    const inside = pointer.x >= rect.left && pointer.x <= rect.right && pointer.y >= rect.top && pointer.y <= rect.bottom;
    if (inside && interactive && (revealOnHover || visible)) {
      chromeAttachment?.hold();
      hovered = true;
      return;
    }
    if (!inside && hovered) {
      hovered = false;
      if (!focused) chromeAttachment?.release();
    }
  }

  function schedulePointerSync(): void {
    if (!chromeAttachment) return;
    if (pointerSyncFrame !== undefined) cancelAnimationFrame(pointerSyncFrame);
    pointerSyncFrame = requestAnimationFrame(syncPointer);
  }

  $effect(() => {
    void top;
    void layoutOriginOffset;
    void interactive;
    void presented;
    if (chromeAttachment && controlsElement) schedulePointerSync();
  });

  function handlePointerEnter(event: PointerEvent): void {
    if (!controls.enabled) return;
    if (!chromeDiscoverable && !attached && !visible) return;
    if (!revealOnHover && !visible) return;
    if (chromeDiscoverable || attached) chromeAttachment?.hold(event);
    hovered = true;
  }

  function handlePointerLeave(): void {
    if (hovered) showTemporarily();
    hovered = false;
    if (!focused) chromeAttachment?.release();
  }

  function handlePointerMove(event: PointerEvent): void {
    if (!chromeAttachment) return;
    if (!chromeDiscoverable && !attached) return;
    event.stopPropagation();
    chromeAttachment.hold(event);
  }

  function handleFocusIn(): void {
    if (chromeDiscoverable || attached) chromeAttachment?.hold();
    focused = true;
  }

  function handleFocusOut(event: FocusEvent & { currentTarget: HTMLDivElement }): void {
    const nextInside = event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget);
    if (!nextInside && focused) showTemporarily();
    focused = nextInside;
    if (!nextInside && !hovered) chromeAttachment?.release();
  }

  $effect(() => {
    if (!controls.enabled) clearVisibility();
  });

  $effect(() => () => {
    if (pointerSyncFrame !== undefined) cancelAnimationFrame(pointerSyncFrame);
    chromeAttachment?.release();
    clearVisibility();
  });
</script>

<div
  bind:this={anchor}
  style:position={fixed ? 'fixed' : 'absolute'}
  style:top
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
    bind:this={controlsElement}
    style:color={token(engaged ? 'colors.text.default' : 'colors.text.muted')}
    style:opacity={presented ? '1' : '0'}
    style:pointer-events={interactive ? 'auto' : 'none'}
    style:border-radius={borderRadius}
    style:transition={prefersReducedMotion.current
      ? 'none'
      : `opacity ${presented ? FADE_IN_MS : FADE_OUT_MS}ms ease-out, color ${FADE_IN_MS}ms ease-out`}
    class={css({ position: 'relative', isolation: 'isolate', width: '[fit-content]' })}
    data-floating-editor-zoom-attached={attached}
    data-floating-editor-zoom-controls
    data-floating-editor-zoom-hover-reveal={revealOnHover}
    data-pane-chrome-reveal-exclusion
    onfocusin={handleFocusIn}
    onfocusout={handleFocusOut}
    onpointerenter={handlePointerEnter}
    onpointerleave={handlePointerLeave}
    onpointermove={handlePointerMove}
    role="presentation"
  >
    <div
      style:backdrop-filter={`blur(${presented ? 2 : 0}px)`}
      style:border-radius={borderRadius}
      style:transition={prefersReducedMotion.current ? 'none' : 'backdrop-filter 320ms cubic-bezier(0, 0, 0.2, 1)'}
      class={css({ position: 'absolute', inset: '0', zIndex: '[-2]', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-blur
    ></div>
    <div
      style:background-color={TRANSIENT_SURFACE}
      style:border-radius={borderRadius}
      class={css({ position: 'absolute', inset: '0', zIndex: '[-1]', pointerEvents: 'none' })}
      aria-hidden="true"
      data-floating-editor-zoom-surface
    ></div>
    <EditorZoomControls
      {...controls}
      keyboardDiscoverableWhenHidden={!chromeAttachment || chromeDiscoverable || presented}
      onFeedbackVisibilityHoldChange={(held) => (feedbackVisibilityHeld = held)}
      onTemporaryVisibilityRequest={showTemporarily}
      visible={presented}
    />
  </div>
</div>
