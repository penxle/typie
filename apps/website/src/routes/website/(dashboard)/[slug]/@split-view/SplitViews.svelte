<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { clamp } from '@typie/ui/utils';
  import { getSplitViewContext } from './context.svelte';
  import Resizer from './Resizer.svelte';
  import SplitViews from './SplitViews.svelte';
  import { getMinSizeForView, VIEW_MIN_SIZE } from './utils';
  import View from './View.svelte';
  import type { SplitViews_View_query } from '$graphql';
  import type { SplitView } from './context.svelte';

  type Props = {
    $query: SplitViews_View_query;
    slug: string;
    view: SplitView;
  };

  let { $query: _query, slug, view }: Props = $props();

  const context = getSplitViewContext();
  let containerRef: HTMLDivElement | undefined = $state();

  const sizePercentage = $derived.by(() => {
    const v = context.state.current.currentPercentages?.[view.id];
    return v == null || Number.isNaN(v) ? 100 : clamp(v, 0, 100);
  });

  const minParallelSize = $derived.by(() => {
    if (view.type === 'item') {
      return VIEW_MIN_SIZE;
    }
    return getMinSizeForView(view, view.direction);
  });
  const minPerpendicularSize = $derived.by(() => {
    if (view.type === 'item') {
      return VIEW_MIN_SIZE;
    }
    return getMinSizeForView(view, view.direction === 'horizontal' ? 'vertical' : 'horizontal');
  });
</script>

{#if view.type === 'item'}
  <View $query={_query} viewItem={view} />
{:else if view.type === 'container'}
  <div
    bind:this={containerRef}
    style:flex-basis={`${sizePercentage}%`}
    style:min-width={view.direction === 'horizontal' ? `${minParallelSize}px` : `${minPerpendicularSize}px`}
    style:min-height={view.direction === 'vertical' ? `${minParallelSize}px` : `${minPerpendicularSize}px`}
    class={flex({
      flexDirection: view.direction === 'horizontal' ? 'row' : 'column',
      flex: '1',
      size: 'full',
      backgroundColor: 'surface.muted',
      overflowX: view.direction === 'horizontal' ? 'auto' : 'hidden',
      overflowY: view.direction === 'vertical' ? 'auto' : 'hidden',
      alignItems: 'stretch',
      position: 'relative',
    })}
  >
    {#each view.children as child, index (child.type === 'item' ? `item-${child.id}-${child.slug ?? ''}` : `container-${child.id}`)}
      <SplitViews $query={_query} {slug} view={child} />
      {#if index !== view.children.length - 1}
        <Resizer {containerRef} direction={view.direction} {index} {view} />
      {/if}
    {/each}
  </div>
{/if}
