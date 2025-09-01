<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import SplitViews from './SplitViews.svelte';
  import View from './View.svelte';
  import type { SplitViews_View_query } from '$graphql';
  import type { SplitView } from './context.svelte';

  type Props = {
    $query: SplitViews_View_query;
    slug: string;
    view: SplitView;
  };

  let { $query: _query, slug, view }: Props = $props();
</script>

{#if view.type === 'item'}
  <View $query={_query} viewItem={view} />
{:else if view.type === 'container'}
  <div
    class={flex({
      flexDirection: view.direction === 'horizontal' ? 'row' : 'column',
      gap: '4px',
      flex: '1',
      size: 'full',
      backgroundColor: 'surface.muted',
      overflow: 'auto',
    })}
  >
    {#each view.children as child (child.id)}
      <SplitViews $query={_query} {slug} view={child} />
    {/each}
  </div>
{/if}
