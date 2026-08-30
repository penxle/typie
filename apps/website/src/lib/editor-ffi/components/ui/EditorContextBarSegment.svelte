<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { token } from '@typie/styled-system/tokens';
  import { hoverIntent } from '@typie/ui/actions';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { CONTEXT_BAR_FADE_IN_MS, CONTEXT_BAR_FADE_OUT_MS, CONTEXT_BAR_TRANSIENT_VISIBLE_MS } from './editor-context-bar.svelte';
  import type { Snippet } from 'svelte';
  import type { ContextBarSegmentPresentation, EditorContextBarSegmentState } from './editor-context-bar.svelte';

  type Props = {
    id: 'breadcrumb' | 'view-controls';
    state: EditorContextBarSegmentState;
    presentation: ContextBarSegmentPresentation;
    interactiveWhenHidden?: boolean;
    element?: HTMLElement;
    children: Snippet;
  };

  const HOVER_INTENT_DELAY_MS = 400;

  let { id, state, presentation, interactiveWhenHidden = false, element = $bindable(), children }: Props = $props();
  let hoverIntended = false;

  function handlePointerEnter() {
    hoverIntended = false;
    if (presentation.visible) state.setHovered(true);
  }

  function handleHoverIntent() {
    hoverIntended = true;
    state.setHovered(true);
  }

  function handlePointerLeave() {
    const shouldLinger = state.hovered && hoverIntended;
    state.setHovered(false);
    if (shouldLinger) state.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    hoverIntended = false;
  }

  function handleFocusIn() {
    state.setFocused(true);
  }

  function handleFocusOut(event: FocusEvent & { currentTarget: HTMLDivElement }) {
    const nextInside = event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget);
    if (!nextInside && state.focused) state.showTemporarily(CONTEXT_BAR_TRANSIENT_VISIBLE_MS);
    state.setFocused(nextInside);
  }
</script>

<div
  bind:this={element}
  style:opacity={presentation.visible ? '1' : '0'}
  style:color={token(presentation.tone === 'engaged' ? 'colors.text.default' : 'colors.text.subtle')}
  style:pointer-events={interactiveWhenHidden || presentation.visible ? 'auto' : 'none'}
  style:transition={prefersReducedMotion.current
    ? 'none'
    : `opacity ${presentation.visible ? CONTEXT_BAR_FADE_IN_MS : CONTEXT_BAR_FADE_OUT_MS}ms ease-out, color ${CONTEXT_BAR_FADE_IN_MS}ms ease-out`}
  class={css({ position: 'relative', zIndex: '1', minWidth: '0' })}
  data-context-bar-segment={id}
  data-context-bar-tone={presentation.tone}
  onfocusin={handleFocusIn}
  onfocusout={handleFocusOut}
  role="presentation"
  use:hoverIntent={{
    delay: HOVER_INTENT_DELAY_MS,
    samples: 1,
    onEnter: handlePointerEnter,
    onIntent: handleHoverIntent,
    onLeave: handlePointerLeave,
  }}
>
  {@render children()}
</div>
