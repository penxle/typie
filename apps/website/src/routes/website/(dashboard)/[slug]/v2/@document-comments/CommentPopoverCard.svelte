<script lang="ts">
  import { flip, hide, inline, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createCenterWhenReferenceDoesNotFitMiddleware, createFloatingActions } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import { getCommentContext } from './context.svelte';
  import type { ReferenceElement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';

  type Props = {
    boundary: HTMLElement;
    reference: ReferenceElement;
    onclickoutside: () => void;
    children: Snippet;
  };
  let { boundary, reference, onclickoutside, children }: Props = $props();

  const comments = getCommentContext();
  let element = $state<HTMLElement | null>(null);

  const POPOVER_GAP = 6;
  const VIEWPORT_PADDING = 8;
  const centeredFallback = createCenterWhenReferenceDoesNotFitMiddleware({
    gap: POPOVER_GAP,
    overflow: () => ({ boundary, padding: VIEWPORT_PADDING }),
  });

  const { anchor: referenceAction, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: POPOVER_GAP,
    middleware: [
      centeredFallback.captureReferenceBounds,
      inline(),
      flip({
        get boundary() {
          return boundary;
        },
        padding: VIEWPORT_PADDING,
      }),
      shift({
        get boundary() {
          return boundary;
        },
        padding: VIEWPORT_PADDING,
      }),
      centeredFallback.centerWhenNeitherSideFits,
      hide({
        get boundary() {
          return boundary;
        },
      }),
    ],
    onClickOutside: (event: Event) => {
      if (event.target instanceof HTMLElement && event.target.closest('[data-comment-panel-item]')) return;
      onclickoutside();
    },
  });

  $effect(() => {
    referenceAction(reference);
  });

  $effect(() => {
    comments.setFocusReturnRegion(element);
    return () => comments.setFocusReturnRegion(null);
  });
</script>

<div
  bind:this={element}
  style:width="280px"
  class={css({
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    boxShadow: 'small',
    zIndex: 'menu',
    pointerEvents: 'auto',
    overflow: 'hidden',
    transformOrigin: 'top left',
  })}
  onclick={(e) => e.stopPropagation()}
  onfocusin={(event) => {
    if (event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)) return;
    comments.captureFocusReturn(event.relatedTarget);
  }}
  role="presentation"
  use:floating={{ appendTo: boundary }}
  transition:scale={{ start: 0.95, duration: 150 }}
>
  {@render children()}
</div>
