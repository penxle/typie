<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Scrollbar } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { tick, untrack } from 'svelte';
  import { smootherstep } from './editor-context-bar.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    pathIdentity: string;
    viewportId: string;
    onPathChange?: () => void;
    children: Snippet;
  };

  type Metrics = {
    viewportWidth: number;
    contentWidth: number;
  };

  const FOG_WIDTH = 24;
  const FOG_SAMPLES = 6;
  const FOG_TRANSITION_MS = 160;
  const AT_END_EPSILON = 1;

  let { pathIdentity, viewportId, onPathChange, children }: Props = $props();
  let viewport = $state<HTMLElement>();
  let content = $state<HTMLElement>();
  let metrics = $state<Metrics>({ viewportWidth: 0, contentWidth: 0 });
  let overflowLeading = $state(false);
  let overflowTrailing = $state(false);
  let pendingTrailingAlignment = true;
  let lastPathIdentity: string | undefined;
  let pathRevision = 0;
  let fogTransitionReady = $state(false);
  let fogTransitionFrame: number | undefined;
  let trailingAlignmentFrame: number | undefined;

  const leadingMaskStops = Array.from({ length: FOG_SAMPLES + 1 }, (_, index) => {
    const progress = index / FOG_SAMPLES;
    const alphaGain = smootherstep(progress);
    return `rgb(0 0 0 / calc(1 - var(--breadcrumb-leading-fog) * ${1 - alphaGain})) ${(index / FOG_SAMPLES) * FOG_WIDTH}px`;
  });
  const trailingMaskStops = Array.from({ length: FOG_SAMPLES + 1 }, (_, index) => {
    const offset = (index / FOG_SAMPLES) * FOG_WIDTH;
    const alphaLoss = smootherstep(index / FOG_SAMPLES);
    return `rgb(0 0 0 / calc(1 - var(--breadcrumb-trailing-fog) * ${alphaLoss})) calc(100% - ${FOG_WIDTH - offset}px)`;
  });
  const maskImage = `linear-gradient(to right, ${leadingMaskStops.join(', ')}, black ${FOG_WIDTH}px, black calc(100% - ${FOG_WIDTH}px), ${trailingMaskStops.join(', ')})`;

  function updateOverflow() {
    if (!viewport) return;
    const maximum = Math.max(0, viewport.scrollWidth - viewport.clientWidth);
    overflowLeading = viewport.scrollLeft > AT_END_EPSILON;
    overflowTrailing = viewport.scrollLeft < maximum - AT_END_EPSILON;
  }

  function enableFogTransitions() {
    if (fogTransitionReady || fogTransitionFrame !== undefined) return;
    fogTransitionFrame = requestAnimationFrame(() => {
      fogTransitionFrame = requestAnimationFrame(() => {
        fogTransitionFrame = undefined;
        fogTransitionReady = true;
      });
    });
  }

  function finishTrailingAlignment() {
    if (trailingAlignmentFrame !== undefined) return;
    trailingAlignmentFrame = requestAnimationFrame(() => {
      trailingAlignmentFrame = undefined;
      if (!pendingTrailingAlignment || !viewport) return;
      viewport.scrollLeft = Math.max(0, viewport.scrollWidth - viewport.clientWidth);
      pendingTrailingAlignment = false;
      updateOverflow();
    });
  }

  function syncGeometry() {
    if (!viewport || !content) return;

    const nextMetrics = {
      viewportWidth: viewport.clientWidth,
      contentWidth: content.offsetWidth,
    };
    if (nextMetrics.viewportWidth <= 0 || nextMetrics.contentWidth <= 0) {
      metrics = nextMetrics;
      updateOverflow();
      return;
    }

    const firstLayout = metrics.viewportWidth <= 0 || metrics.contentWidth <= 0;
    const viewportChanged = nextMetrics.viewportWidth !== metrics.viewportWidth;
    const contentChanged = nextMetrics.contentWidth !== metrics.contentWidth;
    const previousMaximum = Math.max(0, metrics.contentWidth - metrics.viewportWidth);
    const wasAtTrailingEnd = firstLayout || viewport.scrollLeft >= previousMaximum - AT_END_EPSILON;
    const nextMaximum = Math.max(0, nextMetrics.contentWidth - nextMetrics.viewportWidth);
    const previousPosition = viewport.scrollLeft;

    metrics = nextMetrics;
    if (pendingTrailingAlignment || firstLayout) {
      viewport.scrollLeft = nextMaximum;
      finishTrailingAlignment();
    } else if (viewportChanged) {
      viewport.scrollLeft = Math.min(previousPosition, nextMaximum);
    } else if (contentChanged && wasAtTrailingEnd) {
      viewport.scrollLeft = nextMaximum;
    } else {
      viewport.scrollLeft = Math.min(previousPosition, nextMaximum);
    }

    updateOverflow();
    enableFogTransitions();
  }

  $effect(() => {
    const nextPathIdentity = pathIdentity;
    if (nextPathIdentity === lastPathIdentity) return;
    const pathChanged = lastPathIdentity !== undefined;
    lastPathIdentity = nextPathIdentity;
    pendingTrailingAlignment = true;
    if (pathChanged) onPathChange?.();
    const revision = ++pathRevision;
    void tick().then(() => {
      if (revision === pathRevision) syncGeometry();
    });
  });

  $effect(() => {
    const currentViewport = viewport;
    const currentContent = content;
    if (!currentViewport || !currentContent) return;

    const observer = new ResizeObserver(syncGeometry);
    observer.observe(currentViewport);
    observer.observe(currentContent);
    untrack(syncGeometry);

    return () => observer.disconnect();
  });

  $effect(() => {
    return () => {
      if (fogTransitionFrame !== undefined) cancelAnimationFrame(fogTransitionFrame);
      if (trailingAlignmentFrame !== undefined) cancelAnimationFrame(trailingAlignmentFrame);
    };
  });
</script>

<div class={css({ position: 'relative', minWidth: '0', maxWidth: 'full', width: '[max-content]' })}>
  <div
    bind:this={viewport}
    id={viewportId}
    style:mask-image={maskImage}
    style:--breadcrumb-leading-fog={overflowLeading ? 1 : 0}
    style:--breadcrumb-trailing-fog={overflowTrailing ? 1 : 0}
    style:transition={fogTransitionReady && !prefersReducedMotion.current
      ? `--breadcrumb-leading-fog ${FOG_TRANSITION_MS}ms ease-out, --breadcrumb-trailing-fog ${FOG_TRANSITION_MS}ms ease-out`
      : 'none'}
    class={css({ width: 'full', minWidth: '0', overflowX: 'auto', overflowY: 'hidden', scrollbarWidth: 'none' })}
    data-breadcrumb-fog-curve="smootherstep"
    data-breadcrumb-fog-leading={overflowLeading}
    data-breadcrumb-fog-trailing={overflowTrailing}
    data-editor-breadcrumb-viewport
    onscroll={updateOverflow}
  >
    <div bind:this={content} class={css({ display: 'inline-flex', minWidth: '[max-content]' })}>
      {@render children()}
    </div>
  </div>

  <Scrollbar controls={viewportId} label="문서 경로 가로 스크롤" orientation="horizontal" scrollContainer={viewport} size="sm" />
</div>

<style>
  @property --breadcrumb-leading-fog {
    syntax: '<number>';
    inherits: false;
    initial-value: 0;
  }

  @property --breadcrumb-trailing-fog {
    syntax: '<number>';
    inherits: false;
    initial-value: 0;
  }
</style>
