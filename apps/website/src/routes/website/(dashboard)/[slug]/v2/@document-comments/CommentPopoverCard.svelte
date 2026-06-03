<script lang="ts">
  import { flip, hide, inline, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import type { ReferenceElement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';

  type Props = {
    reference: ReferenceElement;
    onclickoutside: () => void;
    children: Snippet;
  };
  let { reference, onclickoutside, children }: Props = $props();

  const { anchor: referenceAction, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 6,
    middleware: [inline(), flip(), shift({ padding: 8 }), hide()],
    onClickOutside: (event: Event) => {
      if (event.target instanceof HTMLElement && event.target.closest('[data-comment-panel-item]')) return;
      onclickoutside();
    },
  });

  $effect(() => {
    referenceAction(reference);
  });
</script>

<div
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
  role="presentation"
  use:floating
  transition:scale={{ start: 0.95, duration: 150 }}
>
  {@render children()}
</div>
