<script lang="ts">
  import { getContext } from 'svelte';
  import { css } from '$styled-system/css';
  import type { Snippet } from 'svelte';
  import type { DragEventHandler } from 'svelte/elements';
  import type { SystemStyleObject } from '$styled-system/types';

  type Props = {
    as?: keyof HTMLElementTagNameMap;
    style?: SystemStyleObject;
    children: Snippet;
    [key: string]: unknown;
  };

  let { as = 'div', style, children, ...rest }: Props = $props();

  const onDragStart = getContext<DragEventHandler<HTMLDivElement>>('onDragStart');
</script>

<svelte:element
  this={as}
  class={css({ fontFamily: 'ui', whiteSpace: 'normal', userSelect: 'none' }, style)}
  data-node-view
  ondragstart={onDragStart}
  role="presentation"
  {...rest}
>
  {@render children()}
</svelte:element>
