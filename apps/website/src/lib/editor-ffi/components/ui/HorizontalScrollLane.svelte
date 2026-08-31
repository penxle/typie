<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { scrollFog } from '@typie/ui/actions';
  import { Scrollbar } from '@typie/ui/components';
  import { tick, untrack } from 'svelte';
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

  const AT_END_EPSILON = 1;

  let { alignment = 'start', children, contentIdentity, label, onContentChange, viewportId, viewportName }: Props = $props();
  let viewport = $state<HTMLElement>();
  let content = $state<HTMLElement>();
  let metrics = $state<Metrics>({ viewportWidth: 0, contentWidth: 0 });
  let pendingAlignment = true;
  let lastContentIdentity: string | undefined;
  let contentRevision = 0;
  let alignmentFrame: number | undefined;

  function maximumScroll() {
    if (!viewport) return 0;
    return Math.max(0, viewport.scrollWidth - viewport.clientWidth);
  }

  function alignedPosition(maximum: number) {
    return alignment === 'end' ? maximum : 0;
  }

  function finishAlignment() {
    if (alignmentFrame !== undefined) return;
    alignmentFrame = requestAnimationFrame(() => {
      alignmentFrame = undefined;
      if (!pendingAlignment || !viewport) return;
      viewport.scrollLeft = alignedPosition(maximumScroll());
      pendingAlignment = false;
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
      if (alignmentFrame !== undefined) cancelAnimationFrame(alignmentFrame);
    };
  });
</script>

<div class={css({ position: 'relative', minWidth: '0', maxWidth: 'full', width: '[max-content]', height: 'full' })}>
  <div
    bind:this={viewport}
    id={viewportId}
    class={css({ width: 'full', height: 'full', minWidth: '0', overflowX: 'auto', overflowY: 'hidden', scrollbarWidth: 'none' })}
    data-horizontal-scroll-viewport={viewportName}
    use:scrollFog={{ orientation: 'horizontal' }}
  >
    <div bind:this={content} class={css({ display: 'inline-flex', alignItems: 'center', minWidth: '[max-content]', height: 'full' })}>
      {@render children()}
    </div>
  </div>

  <Scrollbar controls={viewportId} {label} orientation="horizontal" scrollContainer={viewport} size="sm" />
</div>
