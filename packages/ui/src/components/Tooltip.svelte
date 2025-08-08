<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { writable } from 'svelte/store';
  import { scale } from 'svelte/transition';
  import { createFloatingActions, hover } from '../actions';
  import type { Placement } from '@floating-ui/dom';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    message?: string | Snippet;
    style?: SystemStyleObject;
    tooltipStyle?: SystemStyleObject;
    offset?: number;
    enabled?: boolean;
    placement?: Placement;
    keepShowing?: boolean;
    children: Snippet;
  };

  let { message, style, tooltipStyle, offset, enabled = true, placement = 'bottom', keepShowing = false, children }: Props = $props();

  const hovered = writable(false);

  const { anchor, floating, arrow } = createFloatingActions({
    placement,
    offset: offset ?? 6,
    arrow: true,
  });
</script>

<div class={css(style)} use:anchor use:hover={hovered}>
  {@render children()}
</div>

{#if enabled && ($hovered || keepShowing)}
  <div
    class={css(
      {
        borderRadius: '4px',
        paddingX: '8px',
        paddingY: '4px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: 'text.bright',
        backgroundColor: 'surface.dark',
        boxShadow: 'medium',
        zIndex: 'tooltip',
        pointerEvents: 'none',
      },
      tooltipStyle,
    )}
    role="tooltip"
    use:floating
    transition:scale|global={{ start: 0.9, duration: 200 }}
  >
    {#if typeof message === 'string'}
      {message}
    {:else}
      {@render message?.()}
    {/if}
    <div
      class={css({
        borderTopLeftRadius: '2px',
        size: '8px',
        backgroundColor: 'surface.dark',
      })}
      use:arrow
    ></div>
  </div>
{/if}
