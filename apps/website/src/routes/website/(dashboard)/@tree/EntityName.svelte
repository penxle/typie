<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { hoverIntent, scrollFog } from '@typie/ui/actions';
  import { prefersReducedMotion as reducedMotionPreference } from '@typie/ui/state';
  import { onDestroy, onMount } from 'svelte';
  import { advanceEntityNameMotion } from './entity-name-motion';

  type Props = {
    name: string;
    active?: boolean;
  };

  let { name, active = false }: Props = $props();

  const HOVER_DELAY = 400;

  let viewport = $state<HTMLSpanElement>();
  let viewportWidth = $state(0);
  let contentWidth = $state(0);
  const prefersReducedMotion = $derived(reducedMotionPreference.current);
  let animationFrame: number | undefined;
  let hovered = false;
  let hoverReady = $state(false);
  let motionPosition = 0;
  let motionVelocity = 0;
  let previousFrameTime: number | undefined;

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
  };

  const reset = () => {
    cancelAnimation();
    motionPosition = 0;
    if (viewport) viewport.scrollLeft = 0;
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

      if (next.position < maximum) {
        animationFrame = window.requestAnimationFrame(advance);
      } else {
        motionVelocity = 0;
        previousFrameTime = undefined;
      }
    };

    animationFrame = window.requestAnimationFrame(advance);
  };

  $effect(() => {
    void name;
    reset();
    if (hoverReady) startMotion();
  });

  $effect(() => {
    void viewportWidth;
    void contentWidth;
    if (hoverReady) startMotion();
  });

  $effect(() => {
    if (!prefersReducedMotion || !hoverReady || !viewport) return;
    settleAt(viewport, Math.max(0, viewport.scrollWidth - viewport.clientWidth));
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
    return () => {
      hoverIntentAction?.destroy?.();
    };
  });

  onDestroy(cancelAnimation);
</script>

<span
  bind:this={viewport}
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
  bind:clientWidth={viewportWidth}
  use:scrollFog={{ orientation: 'horizontal' }}
>
  <span class={css({ display: 'inline-block' })} bind:offsetWidth={contentWidth}>{name}</span>
</span>
