<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = { shown: boolean; children: Snippet };

  let { shown, children }: Props = $props();

  let content = $state(0);

  const hostClass = css({
    flexShrink: '0',
    overflow: 'hidden',
    opacity: '0',
    transform: '[translateY(8px)]',
    transition: '[height 300ms cubic-bezier(0.23, 1, 0.32, 1), opacity 260ms ease-out, transform 300ms cubic-bezier(0.23, 1, 0.32, 1)]',
    willChange: 'height, opacity, transform',
    '&[data-shown="true"]': { opacity: '100', transform: '[translateY(0)]' },
    _motionReduce: { transition: '[none]' },
  });
  const padClass = css({ paddingTop: '20px' });
</script>

<div style:height="{shown ? content : 0}px" class={hostClass} data-shown={shown}>
  <div class={padClass} bind:clientHeight={content}>
    {@render children()}
  </div>
</div>
