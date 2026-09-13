<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import { currentSpaceSlug } from '../../../current-space-slug';
  import { seriesListPath } from '../../../paths';
  import SeriesView from '../../../SeriesView.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.seriesQuery));
  const space = $derived(query.data.spaceView);
  const collection = $derived(space.collection);
</script>

<Helmet description={collection.description ?? space.name} title={collection.name} trailing={space.name} />

<nav class={flex({ alignItems: 'center', gap: '6px', flexWrap: 'wrap', marginBottom: '20px', fontSize: '13px', color: 'text.hint' })}>
  <a class={css({ transition: 'colors', _hover: { color: 'text.muted' } })} href={seriesListPath(currentSpaceSlug())}>시리즈</a>
  <span>/</span>
  <span class={css({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })} aria-current="page">
    {collection.name}
  </span>
</nav>

{#key collection.id}
  <SeriesView collectionView$key={collection} />
{/key}
