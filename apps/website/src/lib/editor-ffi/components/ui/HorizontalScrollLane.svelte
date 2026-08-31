<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Scrollbar } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { tick, untrack } from 'svelte';
  import { smootherstep } from './editor-context-bar.svelte';
  import type { Snippet } from 'svelte';

  type Metrics = {
    viewportWidth: number;
    contentWidth: number;
  };

  type Props = {
    alignment?: 'start' | 'end';
    children: Snippet;
    contentIdentity: string;
    label: string;
    onContentChange?: () => void;
    viewportId: string;
    viewportName: string;
  };

  const FOG_WIDTH = 24;
  const FOG_SAMPLES = 6;
  const FOG_TRANSITION_MS = 160;
  const AT_END_EPSILON = 1;

  let { alignment = 'start', children, contentIdentity, label, onContentChange, viewportId, viewportName }: Props = $props();
  let viewport = $state<HTMLElement>();
  let content = $state<HTMLElement>();
  let metrics = $state<Metrics>({ viewportWidth: 0, contentWidth: 0 });
  let overflowLeading = $state(false);
  let overflowTrailing = $state(false);
  let pendingAlignment = true;
  let lastContentIdentity: string | undefined;
  let contentRevision = 0;
  let fogTransitionReady = $state(false);
  let fogTransitionFrame: number | undefined;
  let alignmentFrame: number | undefined;

  const leadingMaskStops = Array.from({ length: FOG_SAMPLES + 1 }, (_, index) => {
    const progress = index / FOG_SAMPLES;
    const alphaGain = smootherstep(progress);
    return `rgb(0 0 0 / calc(1 - var(--horizontal-scroll-leading-fog) * ${1 - alphaGain})) ${(index / FOG_SAMPLES) * FOG_WIDTH}px`;
  });
  const trailingMaskStops = Array.from({ length: FOG_SAMPLES + 1 }, (_, index) => {
    const offset = (index / FOG_SAMPLES) * FOG_WIDTH;
    const alphaLoss = smootherstep(index / FOG_SAMPLES);
    return `rgb(0 0 0 / calc(1 - var(--horizontal-scroll-trailing-fog) * ${alphaLoss})) calc(100% - ${FOG_WIDTH - offset}px)`;
  });
  const maskImage = `linear-gradient(to right, ${leadingMaskStops.join(', ')}, black ${FOG_WIDTH}px, black calc(100% - ${FOG_WIDTH}px), ${trailingMaskStops.join(', ')})`;

  function maximumScroll() {
    if (!viewport) return 0;
    return Math.max(0, viewport.scrollWidth - viewport.clientWidth);
  }

  function alignedPosition(maximum: number) {
    return alignment === 'end' ? maximum : 0;
  }

  function updateOverflow() {
    if (!viewport) return;
    const maximum = maximumScroll();
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

  function finishAlignment() {
    if (alignmentFrame !== undefined) return;
    alignmentFrame = requestAnimationFrame(() => {
      alignmentFrame = undefined;
      if (!pendingAlignment || !viewport) return;
      viewport.scrollLeft = alignedPosition(maximumScroll());
      pendingAlignment = false;
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
    const wasAligned =
      firstLayout ||
      (alignment === 'end' ? viewport.scrollLeft >= previousMaximum - AT_END_EPSILON : viewport.scrollLeft <= AT_END_EPSILON);
    const nextMaximum = Math.max(0, nextMetrics.contentWidth - nextMetrics.viewportWidth);
    const previousPosition = viewport.scrollLeft;

    metrics = nextMetrics;
    if (pendingAlignment || firstLayout) {
      viewport.scrollLeft = alignedPosition(nextMaximum);
      finishAlignment();
    } else if (viewportChanged) {
      viewport.scrollLeft = Math.min(previousPosition, nextMaximum);
    } else if (contentChanged && wasAligned) {
      viewport.scrollLeft = alignedPosition(nextMaximum);
    } else {
      viewport.scrollLeft = Math.min(previousPosition, nextMaximum);
    }

    updateOverflow();
    enableFogTransitions();
  }

  $effect(() => {
    const nextContentIdentity = contentIdentity;
    if (nextContentIdentity === lastContentIdentity) return;
    const contentChanged = lastContentIdentity !== undefined;
    lastContentIdentity = nextContentIdentity;
    pendingAlignment = true;
    if (contentChanged) onContentChange?.();
    const revision = ++contentRevision;
    void tick().then(() => {
      if (revision === contentRevision) syncGeometry();
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
      if (alignmentFrame !== undefined) cancelAnimationFrame(alignmentFrame);
    };
  });
</script>

<div class={css({ position: 'relative', minWidth: '0', maxWidth: 'full', width: '[max-content]', height: 'full' })}>
  <div
    bind:this={viewport}
    id={viewportId}
    style:mask-image={maskImage}
    style:--horizontal-scroll-leading-fog={overflowLeading ? 1 : 0}
    style:--horizontal-scroll-trailing-fog={overflowTrailing ? 1 : 0}
    style:transition={fogTransitionReady && !prefersReducedMotion.current
      ? `--horizontal-scroll-leading-fog ${FOG_TRANSITION_MS}ms ease-out, --horizontal-scroll-trailing-fog ${FOG_TRANSITION_MS}ms ease-out`
      : 'none'}
    class={css({ width: 'full', height: 'full', minWidth: '0', overflowX: 'auto', overflowY: 'hidden', scrollbarWidth: 'none' })}
    data-horizontal-scroll-fog-curve="smootherstep"
    data-horizontal-scroll-fog-leading={overflowLeading}
    data-horizontal-scroll-fog-trailing={overflowTrailing}
    data-horizontal-scroll-viewport={viewportName}
    onscroll={updateOverflow}
  >
    <div bind:this={content} class={css({ display: 'inline-flex', alignItems: 'center', minWidth: '[max-content]', height: 'full' })}>
      {@render children()}
    </div>
  </div>

  <Scrollbar controls={viewportId} {label} orientation="horizontal" scrollContainer={viewport} size="sm" />
</div>

<style>
  @property --horizontal-scroll-leading-fog {
    syntax: '<number>';
    inherits: false;
    initial-value: 0;
  }

  @property --horizontal-scroll-trailing-fog {
    syntax: '<number>';
    inherits: false;
    initial-value: 0;
  }
</style>
