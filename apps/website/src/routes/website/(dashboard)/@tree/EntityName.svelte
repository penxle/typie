<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { hoverIntent } from '@typie/ui/actions';
  import { onDestroy, onMount } from 'svelte';
  import { advanceEntityNameMotion } from './entity-name-motion';

  type Props = {
    name: string;
    active?: boolean;
  };

  let { name, active = false }: Props = $props();

  const FOG_WIDTH = 24;
  const FOG_TRANSITION_DURATION = 160;
  const HOVER_DELAY = 400;

  let viewport = $state<HTMLSpanElement>();
  let viewportWidth = $state(0);
  let contentWidth = $state(0);
  let overflowLeft = $state(false);
  let overflowRight = $state(false);
  let fogTransitionReady = $state(false);
  let prefersReducedMotion = $state(false);
  let animationFrame: number | undefined;
  let fogTransitionFrame: number | undefined;
  let fogTransitionScheduled = false;
  let hovered = false;
  let hoverReady = false;
  let motionPosition = 0;
  let motionVelocity = 0;
  let previousFrameTime: number | undefined;

  const updateOverflow = () => {
    overflowLeft = (viewport?.scrollLeft ?? 0) > 1;
    overflowRight = viewport ? viewport.scrollLeft + viewport.clientWidth < viewport.scrollWidth - 1 : false;
  };

  const cancelAnimation = () => {
    if (animationFrame !== undefined) {
      window.cancelAnimationFrame(animationFrame);
      animationFrame = undefined;
    }
    motionVelocity = 0;
    previousFrameTime = undefined;
  };

  const settleAt = (element: HTMLSpanElement, position: number) => {
    cancelAnimation();
    motionPosition = position;
    element.scrollLeft = position;
    updateOverflow();
  };

  const reset = () => {
    cancelAnimation();
    motionPosition = 0;
    if (viewport) viewport.scrollLeft = 0;
    updateOverflow();
  };

  const startMotion = () => {
    const element = viewport;
    if (!hovered || !hoverReady || !element) return;

    const maximumScrollLeft = Math.max(0, element.scrollWidth - element.clientWidth);
    if (prefersReducedMotion || maximumScrollLeft <= element.scrollLeft) {
      settleAt(element, maximumScrollLeft);
      return;
    }

    if (animationFrame !== undefined) return;

    motionPosition = element.scrollLeft;

    const advance = (time: number) => {
      animationFrame = undefined;
      if (!hovered || !hoverReady || viewport !== element) {
        cancelAnimation();
        return;
      }

      const maximum = Math.max(0, element.scrollWidth - element.clientWidth);
      const elapsed = previousFrameTime === undefined ? 0 : time - previousFrameTime;
      previousFrameTime = time;
      const next = advanceEntityNameMotion({ position: motionPosition, velocity: motionVelocity, maximum, elapsed });

      motionPosition = next.position;
      element.scrollLeft = next.position;
      motionVelocity = next.velocity;
      updateOverflow();

      if (next.position < maximum) {
        animationFrame = window.requestAnimationFrame(advance);
      } else {
        motionVelocity = 0;
        previousFrameTime = undefined;
      }
    };

    animationFrame = window.requestAnimationFrame(advance);
  };

  const maskImage = `linear-gradient(to right, rgb(0 0 0 / var(--entity-name-leading-mask-alpha)), black ${FOG_WIDTH}px, black calc(100% - ${FOG_WIDTH}px), rgb(0 0 0 / var(--entity-name-trailing-mask-alpha)))`;

  $effect(() => {
    void name;
    reset();
    if (hoverReady) startMotion();
  });

  $effect(() => {
    void viewportWidth;
    void contentWidth;
    updateOverflow();
    if (hoverReady) startMotion();

    if (!fogTransitionScheduled && viewportWidth > 0 && contentWidth > 0) {
      fogTransitionScheduled = true;
      fogTransitionFrame = window.requestAnimationFrame(() => {
        fogTransitionFrame = window.requestAnimationFrame(() => {
          fogTransitionFrame = undefined;
          fogTransitionReady = true;
        });
      });
    }
  });

  onMount(() => {
    const hoverTarget = viewport?.closest<HTMLElement>('[role="treeitem"]');
    const hoverIntentAction = hoverTarget
      ? hoverIntent(hoverTarget, {
          delay: HOVER_DELAY,
          onEnter: () => {
            hovered = true;
          },
          onIntent: () => {
            hoverReady = true;
            startMotion();
          },
          onLeave: () => {
            hovered = false;
            hoverReady = false;
            reset();
          },
        })
      : undefined;
    const reducedMotionQuery = window.matchMedia('(prefers-reduced-motion: reduce)');
    const updateReducedMotion = () => {
      prefersReducedMotion = reducedMotionQuery.matches;
      const element = viewport;
      if (!prefersReducedMotion || !hoverReady || !element) return;

      settleAt(element, Math.max(0, element.scrollWidth - element.clientWidth));
    };

    reducedMotionQuery.addEventListener('change', updateReducedMotion);
    updateReducedMotion();
    updateOverflow();

    return () => {
      if (fogTransitionFrame !== undefined) window.cancelAnimationFrame(fogTransitionFrame);
      hoverIntentAction?.destroy?.();
      reducedMotionQuery.removeEventListener('change', updateReducedMotion);
    };
  });

  onDestroy(cancelAnimation);
</script>

<span
  bind:this={viewport}
  style:mask-image={maskImage}
  style:--entity-name-leading-mask-alpha={overflowLeft ? 0 : 1}
  style:--entity-name-trailing-mask-alpha={overflowRight ? 0 : 1}
  style:transition={fogTransitionReady && !prefersReducedMotion
    ? `--entity-name-leading-mask-alpha ${FOG_TRANSITION_DURATION}ms ease-in-out, --entity-name-trailing-mask-alpha ${FOG_TRANSITION_DURATION}ms ease-in-out`
    : 'none'}
  class={css(
    {
      display: 'block',
      flexGrow: '1',
      minWidth: '0',
      marginX: '-6px',
      paddingX: '6px',
      overflowX: 'hidden',
      whiteSpace: 'nowrap',
      scrollbarWidth: 'none',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'text.muted',
    },
    active && { fontWeight: 'bold', color: 'text.default' },
  )}
  onscroll={updateOverflow}
  bind:clientWidth={viewportWidth}
>
  <span class={css({ display: 'inline-block' })} bind:offsetWidth={contentWidth}>{name}</span>
</span>

<style>
  @property --entity-name-leading-mask-alpha {
    syntax: '<number>';
    inherits: false;
    initial-value: 1;
  }

  @property --entity-name-trailing-mask-alpha {
    syntax: '<number>';
    inherits: false;
    initial-value: 1;
  }
</style>
