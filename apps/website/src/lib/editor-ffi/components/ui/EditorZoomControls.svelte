<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion as reducedMotionPreference } from '@typie/ui/state';
  import { fade } from 'svelte/transition';
  import FileCheckCornerIcon from '~icons/lucide/file-check-corner';
  import GalleryHorizontalIcon from '~icons/lucide/gallery-horizontal';
  import MinusIcon from '~icons/lucide/minus';
  import PlusIcon from '~icons/lucide/plus';
  import SearchIcon from '~icons/lucide/search';
  import ZoomInIcon from '~icons/lucide/zoom-in';
  import ZoomOutIcon from '~icons/lucide/zoom-out';
  import { zoomDiffers } from '$lib/editor-ffi/zoom';
  import type { DocumentZoomLandmark } from '$lib/editor-ffi/zoom';

  export type EditorZoomControlsRenderProps = {
    enabled: boolean;
    displayZoom: number;
    indicatorZoom: number;
    landmark: DocumentZoomLandmark | null;
    atMinimum: boolean;
    atMaximum: boolean;
    toggleTargetLandmark: DocumentZoomLandmark | null;
    boundaryAttemptRequest?: number;
    boundaryAttemptLandmark?: DocumentZoomLandmark | null;
    onZoomOut: () => unknown | Promise<unknown>;
    onZoomIn: () => unknown | Promise<unknown>;
    onToggleZoom: () => unknown | Promise<unknown>;
  };

  type Props = EditorZoomControlsRenderProps & {
    visible: boolean;
    keyboardDiscoverableWhenHidden?: boolean;
    onTemporaryVisibilityRequest: (additionalDurationMs?: number) => void;
    onFeedbackVisibilityHoldChange: (held: boolean) => void;
  };

  type ZoomRangeState = 'in-range' | 'below-minimum' | 'above-maximum';

  const ZOOM_LANDMARK_VISIBLE_MS = 1000;
  const ZOOM_VALUE_TRANSITION_MS = 120;
  const DEFAULT_BORDER = token('colors.border.default');
  const LANDMARK_LABELS: Record<DocumentZoomLandmark, string> = {
    minimum: '최소',
    'fit-width': '맞춤',
    unit: '원본',
    maximum: '최대',
  };
  const LANDMARK_ICONS = {
    minimum: ZoomOutIcon,
    'fit-width': GalleryHorizontalIcon,
    unit: FileCheckCornerIcon,
    maximum: ZoomInIcon,
  } as const;

  let {
    enabled,
    displayZoom,
    indicatorZoom,
    landmark,
    atMinimum,
    atMaximum,
    toggleTargetLandmark,
    boundaryAttemptRequest = 0,
    boundaryAttemptLandmark = null,
    visible,
    keyboardDiscoverableWhenHidden = false,
    onTemporaryVisibilityRequest,
    onFeedbackVisibilityHoldChange,
    onZoomOut,
    onZoomIn,
    onToggleZoom,
  }: Props = $props();

  let valueHovered = $state(false);
  let valueFocused = $state(false);
  let announcedLandmark = $state<DocumentZoomLandmark | null>(null);
  let landmarkTimer: ReturnType<typeof setTimeout> | null = null;
  let touchClickSuppressionTimer: ReturnType<typeof setTimeout> | null = null;
  let suppressTouchClick = false;
  let lastZoom: number | null = null;
  let lastLandmark: DocumentZoomLandmark | null | undefined;
  let lastRangeState: ZoomRangeState | undefined;
  let lastBoundaryAttemptRequest = 0;
  let snapFeedbackRequest = $state(0);
  let snapFeedbackLandmark = $state<DocumentZoomLandmark | null>(null);
  const prefersReducedMotion = $derived(reducedMotionPreference.current);

  const zoomPercent = $derived(Math.round(indicatorZoom * 100));
  const displayedLandmark = $derived(valueHovered || valueFocused ? landmark : announcedLandmark);
  const displayedValue = $derived(displayedLandmark ? LANDMARK_LABELS[displayedLandmark] : `${zoomPercent}%`);
  const displayedIcon = $derived(displayedLandmark ? LANDMARK_ICONS[displayedLandmark] : SearchIcon);
  const toggleAvailable = $derived(toggleTargetLandmark !== null);
  const toggleLabel = $derived.by(() => {
    switch (toggleTargetLandmark) {
      case 'fit-width': {
        return '화면에 맞추기';
      }
      case 'unit': {
        return '원본 크기로 돌아가기';
      }
      case 'minimum': {
        return '최소 배율로 축소';
      }
      case 'maximum': {
        return '최대 배율로 확대';
      }
      default: {
        return '원본 크기가 화면에 맞춰져 있어요';
      }
    }
  });
  const toggleKeys = $derived(toggleTargetLandmark === 'unit' ? (['Mod', '0'] as ['Mod', '0']) : undefined);

  function handleValuePointerEnter() {
    valueHovered = true;
  }

  function handleValuePointerLeave() {
    valueHovered = false;
  }

  function handleValueFocus() {
    valueFocused = true;
  }

  function handleValueBlur() {
    valueFocused = false;
  }

  function clearLandmarkTimer() {
    if (!landmarkTimer) return;
    clearTimeout(landmarkTimer);
    landmarkTimer = null;
  }

  function showTemporarily() {
    onTemporaryVisibilityRequest(announcedLandmark ? ZOOM_LANDMARK_VISIBLE_MS : undefined);
  }

  function handleIndicatorPointerDown(event: PointerEvent) {
    if (event.pointerType !== 'touch') {
      event.preventDefault();
      return;
    }
    if (visible) return;
    suppressTouchClick = true;
    showTemporarily();
    event.preventDefault();
    event.stopPropagation();
  }

  function clearTouchClickSuppression() {
    if (touchClickSuppressionTimer) {
      clearTimeout(touchClickSuppressionTimer);
      touchClickSuppressionTimer = null;
    }
    suppressTouchClick = false;
  }

  function handleIndicatorClickCapture(event: MouseEvent) {
    if (!suppressTouchClick) return;
    event.preventDefault();
    event.stopPropagation();
    clearTouchClickSuppression();
  }

  function handleIndicatorPointerUp(event: PointerEvent) {
    if (!suppressTouchClick || event.pointerType !== 'touch') return;
    if (touchClickSuppressionTimer) clearTimeout(touchClickSuppressionTimer);
    touchClickSuppressionTimer = setTimeout(clearTouchClickSuppression, 0);
  }

  function handleIndicatorPointerCancel(event: PointerEvent) {
    if (event.pointerType === 'touch') clearTouchClickSuppression();
  }

  function announceLandmark(next: DocumentZoomLandmark | null, held = false) {
    clearLandmarkTimer();
    announcedLandmark = next;
    onFeedbackVisibilityHoldChange(next !== null && held);
    if (!next || held) return;
    landmarkTimer = setTimeout(() => {
      announcedLandmark = null;
      landmarkTimer = null;
    }, ZOOM_LANDMARK_VISIBLE_MS);
  }

  function resolveZoomRangeState(): ZoomRangeState {
    if (!zoomDiffers(displayZoom, indicatorZoom)) return 'in-range';
    return displayZoom < indicatorZoom ? 'below-minimum' : 'above-maximum';
  }

  function rangeLandmark(state: ZoomRangeState): DocumentZoomLandmark | null {
    if (state === 'below-minimum') return 'minimum';
    if (state === 'above-maximum') return 'maximum';
    return null;
  }

  function requestSnapFeedback(nextLandmark: DocumentZoomLandmark) {
    snapFeedbackLandmark = nextLandmark;
    snapFeedbackRequest += 1;
  }

  function handleToggleZoom() {
    void onToggleZoom();
  }

  $effect(() => {
    const previousZoom = lastZoom;
    lastZoom = displayZoom;
    const initialObservation = previousZoom === null;
    const zoomChanged = !initialObservation && zoomDiffers(previousZoom, displayZoom);
    if (enabled && zoomChanged) showTemporarily();
  });

  $effect(() => {
    const currentRangeState = resolveZoomRangeState();
    const previousRangeState = lastRangeState;
    const currentLandmark = landmark;
    const previousLandmark = lastLandmark;

    if (!enabled) {
      lastRangeState = undefined;
      lastLandmark = undefined;
      clearLandmarkTimer();
      announcedLandmark = null;
      onFeedbackVisibilityHoldChange(false);
      return;
    }

    lastRangeState = currentRangeState;
    lastLandmark = currentLandmark;

    if (previousRangeState === undefined || previousLandmark === undefined) return;

    if (currentRangeState !== previousRangeState) {
      if (currentRangeState !== 'in-range') {
        announceLandmark(rangeLandmark(currentRangeState), true);
        showTemporarily();
      } else if (previousRangeState !== 'in-range') {
        const recoveredLandmark = rangeLandmark(previousRangeState) as DocumentZoomLandmark;
        announceLandmark(recoveredLandmark);
        requestSnapFeedback(recoveredLandmark);
        showTemporarily();
      }
      return;
    }

    if (currentRangeState === 'in-range' && currentLandmark !== previousLandmark) {
      announceLandmark(currentLandmark);
      if (currentLandmark) requestSnapFeedback(currentLandmark);
      showTemporarily();
    }
  });

  $effect(() => {
    const request = boundaryAttemptRequest;
    const requestLandmark = boundaryAttemptLandmark;
    if (request === lastBoundaryAttemptRequest || request <= 0) return;
    lastBoundaryAttemptRequest = request;
    if (requestLandmark !== 'minimum' && requestLandmark !== 'maximum') return;
    if (landmark !== requestLandmark || resolveZoomRangeState() !== 'in-range') return;
    announceLandmark(requestLandmark);
    showTemporarily();
  });

  $effect(() => {
    return () => {
      clearLandmarkTimer();
      clearTouchClickSuppression();
      onFeedbackVisibilityHoldChange(false);
    };
  });
</script>

{#if enabled}
  <div
    class={css({
      display: 'flex',
      alignItems: 'center',
      gap: '2px',
      paddingX: '8px',
      paddingY: '4px',
      fontSize: '12px',
      fontWeight: 'medium',
    })}
    aria-label="페이지 배율"
    onclickcapture={handleIndicatorClickCapture}
    onpointercancel={handleIndicatorPointerCancel}
    onpointerdown={handleIndicatorPointerDown}
    onpointerup={handleIndicatorPointerUp}
    role="group"
  >
    <button
      class={css({
        display: 'grid',
        placeItems: 'center',
        size: '24px',
        cursor: 'pointer',
        borderRadius: '8px',
        _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
        '&[data-at-zoom-boundary="true"]': { color: 'text.muted', _hover: { color: 'text.muted' } },
      })}
      aria-label={atMinimum ? '최소 배율입니다' : '페이지 축소'}
      data-at-zoom-boundary={atMinimum}
      onclick={onZoomOut}
      tabindex={visible || keyboardDiscoverableWhenHidden ? 0 : -1}
      type="button"
      use:tooltip={{
        message: atMinimum ? '최소 배율입니다' : '페이지 축소',
        keys: atMinimum ? undefined : ['Mod', '-'],
        placement: 'bottom',
      }}
    >
      <Icon icon={MinusIcon} size={14} />
    </button>

    <button
      class={css({
        position: 'relative',
        width: '56px',
        height: '24px',
        paddingX: '4px',
        textAlign: 'center',
        cursor: toggleAvailable ? 'pointer' : 'default',
        border: '1px solid transparent',
        borderRadius: '8px',
        _hover: { backgroundColor: 'surface.hover' },
      })}
      aria-label={toggleLabel}
      onblur={handleValueBlur}
      onclick={() => toggleAvailable && handleToggleZoom()}
      onfocus={handleValueFocus}
      onpointerenter={handleValuePointerEnter}
      onpointerleave={handleValuePointerLeave}
      tabindex={visible || keyboardDiscoverableWhenHidden ? 0 : -1}
      type="button"
      use:tooltip={{ message: toggleLabel, keys: toggleKeys, placement: 'bottom' }}
    >
      {#if snapFeedbackRequest > 0 && snapFeedbackLandmark}
        {#key snapFeedbackRequest}
          <span
            style:border-color={DEFAULT_BORDER}
            class="zoom-snap-feedback"
            aria-hidden="true"
            data-reduced-motion={prefersReducedMotion}
            data-zoom-snap-feedback
            data-zoom-snap-landmark={snapFeedbackLandmark}
          ></span>
        {/key}
      {/if}
      <span
        class={css({ position: 'relative', display: 'block', width: 'full', height: '[1.25em]' })}
        aria-live="polite"
        data-zoom-value-kind={displayedLandmark ? 'landmark' : 'percentage'}
      >
        {#key displayedLandmark ? 'landmark' : 'percentage'}
          <span
            class={css({
              position: 'absolute',
              inset: '0',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              gap: '4px',
              whiteSpace: 'nowrap',
            })}
            in:fade={{ duration: prefersReducedMotion ? 0 : ZOOM_VALUE_TRANSITION_MS }}
            out:fade={{ duration: prefersReducedMotion ? 0 : ZOOM_VALUE_TRANSITION_MS }}
          >
            <Icon icon={displayedIcon} size={14} />
            <span>{displayedValue}</span>
          </span>
        {/key}
      </span>
    </button>

    <button
      class={css({
        display: 'grid',
        placeItems: 'center',
        size: '24px',
        cursor: 'pointer',
        borderRadius: '8px',
        _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
        '&[data-at-zoom-boundary="true"]': { color: 'text.muted', _hover: { color: 'text.muted' } },
      })}
      aria-label={atMaximum ? '최대 배율입니다' : '페이지 확대'}
      data-at-zoom-boundary={atMaximum}
      onclick={onZoomIn}
      tabindex={visible || keyboardDiscoverableWhenHidden ? 0 : -1}
      type="button"
      use:tooltip={{
        message: atMaximum ? '최대 배율입니다' : '페이지 확대',
        keys: atMaximum ? undefined : ['Mod', '+'],
        placement: 'bottom',
      }}
    >
      <Icon icon={PlusIcon} size={14} />
    </button>
  </div>
{/if}

<style>
  .zoom-snap-feedback {
    position: absolute;
    inset: -1px;
    z-index: 1;
    border: 2px solid;
    border-radius: 8px;
    pointer-events: none;
    animation: zoom-snap-border-flash 380ms ease-out both;
  }

  .zoom-snap-feedback[data-reduced-motion='true'] {
    border-width: 2px;
    animation: zoom-snap-border-reduced 380ms steps(1) both;
  }

  @keyframes zoom-snap-border-flash {
    0% {
      opacity: 0;
    }
    18% {
      opacity: 1;
    }
    48% {
      opacity: 0.72;
    }
    100% {
      opacity: 0;
    }
  }

  @keyframes zoom-snap-border-reduced {
    0%,
    99% {
      opacity: 0.7;
    }
    100% {
      opacity: 0;
    }
  }
</style>
