<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { onDestroy, untrack } from 'svelte';
  import { hoverIntent } from '../actions/hover-intent.svelte';
  import { scrollFog } from '../actions/scroll-fog.svelte';
  import { prefersReducedMotion as reducedMotionPreference } from '../state/reduced-motion.svelte';
  import { advanceMarqueeMotion } from '../utils/marquee-motion';

  type Bleed = number | { start?: number; end?: number };

  type Props = {
    bleed?: Bleed;
    class?: string;
    fogSize?: number;
    getTrigger?: (element: HTMLElement) => HTMLElement | null;
    text: string;
  };

  const HOVER_DELAY = 400;
  const self = (element: HTMLElement) => element;
  const normalizeBleed = (value: number | undefined) => (value === undefined || !Number.isFinite(value) ? 0 : Math.max(0, value));

  let { bleed, class: className, fogSize, getTrigger = self, text }: Props = $props();

  let viewport = $state<HTMLSpanElement>();
  let viewportWidth = $state(0);
  let contentWidth = $state(0);
  const prefersReducedMotion = $derived(reducedMotionPreference.current);
  const bleedInsets = $derived.by(() => {
    if (typeof bleed === 'number') {
      const inset = normalizeBleed(bleed);
      return { start: inset, end: inset };
    }
    return { start: normalizeBleed(bleed?.start), end: normalizeBleed(bleed?.end) };
  });
  let animationFrame: number | undefined;
  let hovered = false;
  let hoverReady = false;
  let keyboardFocused = false;
  let motionPosition = 0;
  let motionVelocity = 0;
  let previousFrameTime: number | undefined;

  const active = () => (hovered && hoverReady) || keyboardFocused;

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
    if (!element || !active()) return;

    const maximumScrollLeft = Math.max(0, element.scrollWidth - element.clientWidth);
    if (prefersReducedMotion || maximumScrollLeft <= element.scrollLeft) {
      settleAt(element, maximumScrollLeft);
      return;
    }

    if (animationFrame !== undefined) return;

    motionPosition = element.scrollLeft;

    const advance = (time: number) => {
      animationFrame = undefined;
      if (viewport !== element || !active()) {
        cancelAnimation();
        return;
      }

      const maximum = Math.max(0, element.scrollWidth - element.clientWidth);
      const elapsed = previousFrameTime === undefined ? 0 : time - previousFrameTime;
      previousFrameTime = time;
      const next = advanceMarqueeMotion({ position: motionPosition, velocity: motionVelocity, maximum, elapsed });

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
    void text;
    untrack(() => {
      reset();
      if (active()) startMotion();
    });
  });

  $effect(() => {
    void viewportWidth;
    void contentWidth;
    untrack(() => {
      if (active()) startMotion();
    });
  });

  $effect(() => {
    if (!viewport || !prefersReducedMotion || !active()) return;
    settleAt(viewport, Math.max(0, viewport.scrollWidth - viewport.clientWidth));
  });

  $effect(() => {
    const element = viewport;
    if (!element) return;

    const trigger = getTrigger(element);
    if (!trigger) return;

    const action = hoverIntent(trigger, {
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
        if (!keyboardFocused) reset();
      },
    });

    const handleFocusIn = (event: FocusEvent) => {
      const target = event.target;
      keyboardFocused = target instanceof HTMLElement && target.matches(':focus-visible');
      if (keyboardFocused) startMotion();
      else if (!hovered || !hoverReady) reset();
    };
    const handleFocusOut = (event: FocusEvent) => {
      const next = event.relatedTarget;
      if (next instanceof Node && trigger.contains(next)) return;
      keyboardFocused = false;
      if (!hovered || !hoverReady) reset();
    };

    trigger.addEventListener('focusin', handleFocusIn);
    trigger.addEventListener('focusout', handleFocusOut);

    return () => {
      action?.destroy?.();
      trigger.removeEventListener('focusin', handleFocusIn);
      trigger.removeEventListener('focusout', handleFocusOut);
    };
  });

  onDestroy(cancelAnimation);
</script>

<span
  bind:this={viewport}
  style:margin-inline-end={bleedInsets.end > 0 ? `${-bleedInsets.end}px` : undefined}
  style:margin-inline-start={bleedInsets.start > 0 ? `${-bleedInsets.start}px` : undefined}
  style:padding-inline-end={bleedInsets.end > 0 ? `${bleedInsets.end}px` : undefined}
  style:padding-inline-start={bleedInsets.start > 0 ? `${bleedInsets.start}px` : undefined}
  style:text-overflow="clip"
  class={cx(
    css({
      display: 'block',
      minWidth: '0',
      overflowX: 'hidden',
      whiteSpace: 'nowrap',
      scrollbarWidth: 'none',
    }),
    className,
  )}
  bind:clientWidth={viewportWidth}
  use:scrollFog={{ orientation: 'horizontal', size: fogSize }}
>
  <span class={css({ display: 'inline-block' })} bind:offsetWidth={contentWidth}>{text}</span>
</span>
