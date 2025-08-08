<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getContext } from 'svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';
  import type { DragEventHandler } from 'svelte/elements';

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
