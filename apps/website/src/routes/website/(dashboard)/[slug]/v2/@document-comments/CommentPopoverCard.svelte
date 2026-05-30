<script lang="ts">
  import { flip, hide, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import type { Snippet } from 'svelte';

  type Props = {
    x: number;
    y: number;
    onclickoutside: () => void;
    children: Snippet;
  };
  let { x, y, onclickoutside, children }: Props = $props();

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 6,
    middleware: [flip(), shift({ padding: 8 }), hide()],
    onClickOutside: (event: Event) => {
      if (event.target instanceof HTMLElement && event.target.closest('[data-comment-panel-item]')) return;
      onclickoutside();
    },
  });
</script>

<div style:top={`${y}px`} style:left={`${x}px`} class={css({ position: 'absolute' })} use:anchor></div>
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
