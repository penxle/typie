<script lang="ts">
  import { writable } from 'svelte/store';
  import { scale } from 'svelte/transition';
  import { css } from '$styled-system/css';
  import { createFloatingActions, hover } from '../actions';
  import type { Placement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

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
        color: 'text.inverse',
        backgroundColor: 'surface.overlay',
        boxShadow: 'medium',
        zIndex: '50',
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
        backgroundColor: 'surface.overlay',
      })}
      use:arrow
    ></div>
  </div>
{/if}
