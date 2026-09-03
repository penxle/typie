<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { onDestroy } from 'svelte';
  import {
    getZenModePaneChrome,
    PANE_CHROME_EXPANSION_EASING,
    PANE_CHROME_FADE_OUT_MS,
    PANE_CHROME_FOREGROUND_FADE_IN_MS,
    paneChromeExpansionTiming,
  } from './zen-mode-pane-chrome.svelte';
  import type { Snippet } from 'svelte';
  import type { ActionReturn } from 'svelte/action';
  import type { HTMLAttributes } from 'svelte/elements';
  import type { PaneChromeSegment } from './zen-mode-pane-chrome.svelte';

  type Props = Omit<HTMLAttributes<HTMLDivElement>, 'children'> & {
    active: boolean;
    children: Snippet;
    contentCursor?: string;
    register?: (node: HTMLElement) => ActionReturn;
    segment: PaneChromeSegment;
  };

  let { active, children, class: className, contentCursor, register, segment, style, ...attributes }: Props = $props();

  const chrome = getZenModePaneChrome();
  const interactive = $derived(chrome.isInteractive(segment));
  const expansionTiming = $derived(paneChromeExpansionTiming(chrome.phase === 'expanding' ? chrome.expansionPace : 'standard'));
  const registerNode = (node: HTMLElement): ActionReturn => register?.(node) ?? {};
  let hoverHeld = false;
  let focusHeld = false;

  function handleFocusIn(event: FocusEvent): void {
    if (
      event.relatedTarget instanceof Node &&
      event.currentTarget instanceof HTMLElement &&
      event.currentTarget.contains(event.relatedTarget)
    )
      return;
    chrome.recordInteraction();
    if (!focusHeld) chrome.hold(segment, 'focus');
    focusHeld = true;
  }

  function handleFocusOut(event: FocusEvent): void {
    if (
      event.relatedTarget instanceof Node &&
      event.currentTarget instanceof HTMLElement &&
      event.currentTarget.contains(event.relatedTarget)
    )
      return;
    if (focusHeld) chrome.release(segment, 'focus');
    focusHeld = false;
  }

  function handlePointerEnter(): void {
    if (!hoverHeld) chrome.hold(segment, 'hover');
    hoverHeld = true;
  }

  function handlePointerLeave(): void {
    if (hoverHeld) chrome.release(segment, 'hover');
    hoverHeld = false;
  }

  onDestroy(() => {
    if (hoverHeld) chrome.release(segment, 'hover');
    if (focusHeld) chrome.release(segment, 'focus');
  });
</script>

<div
  {...attributes}
  {style}
  style:--zen-pane-chrome-foreground-opacity={chrome.isForegroundRevealStarted(segment) ? 1 : 0}
  style:--zen-pane-chrome-foreground-radius={`${chrome.foregroundRevealRadius(segment === 'toolbar' ? 'toolbar' : 'header')}px`}
  style:clip-path={active ? chrome.foregroundClip(segment) : undefined}
  style:cursor={active && interactive ? contentCursor : undefined}
  style:mask-image={active ? chrome.foregroundMask(segment) : undefined}
  style:opacity={chrome.isForegroundVisible(segment) ? '1' : '0'}
  style:pointer-events={interactive ? 'auto' : 'none'}
  style:transition={active
    ? `opacity ${chrome.phase === 'fading' ? PANE_CHROME_FADE_OUT_MS : PANE_CHROME_FOREGROUND_FADE_IN_MS}ms ease-out, clip-path ${expansionTiming.backgroundExpandMs}ms ${PANE_CHROME_EXPANSION_EASING}, --zen-pane-chrome-foreground-radius ${expansionTiming.backgroundExpandMs}ms ${PANE_CHROME_EXPANSION_EASING}`
    : undefined}
  class={`${css({ position: 'relative', _motionReduce: { transitionDuration: '0ms' } })} ${className ?? ''}`}
  data-pane-chrome-segment={segment}
  inert={!interactive}
  onclickcapture={() => chrome.recordInteraction()}
  onfocusin={handleFocusIn}
  onfocusout={handleFocusOut}
  onpointerenter={handlePointerEnter}
  onpointerleave={handlePointerLeave}
  role="group"
  use:registerNode
>
  {@render children()}
</div>
