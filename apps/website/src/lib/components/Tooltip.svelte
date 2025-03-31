<script lang="ts">
  import { writable } from 'svelte/store';
  import { scale } from 'svelte/transition';
  import { css } from '$styled-system/css';
  import { createFloatingActions, hover } from '../actions';
  import type { Placement } from '@floating-ui/dom';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    message?: string;
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
        borderRadius: '6px',
        paddingX: '12px',
        paddingY: '8px',
        fontSize: '12px',
        fontWeight: 'medium',
        backgroundColor: { base: 'gray.800', _dark: 'gray.500' },
        color: 'white',
        zIndex: '50',
        maxWidth: '220px',
        whiteSpace: 'pre-wrap',
        wordBreak: 'keep-all',
      },
      tooltipStyle,
    )}
    role="tooltip"
    use:floating
    transition:scale={{ start: 0.9, duration: 200 }}
  >
    {message}
    <div
      class={css({
        borderTopLeftRadius: '2px',
        size: '8px',
        backgroundColor: { base: 'gray.800', _dark: 'gray.500' },
      })}
      use:arrow
    ></div>
  </div>
{/if}
