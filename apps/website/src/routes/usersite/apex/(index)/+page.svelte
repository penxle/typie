<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import DiscoveryListSection from '../DiscoveryListSection.svelte';
  import DiscoveryTagChips from '../DiscoveryTagChips.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.feedQuery));
  const discovery = $derived(query.data.discovery);
</script>

<Helmet title="타이피" trailing={null} />

<div class={css({ width: 'full', maxWidth: '[760px]', marginX: 'auto', paddingX: { base: '20px', md: '40px' }, paddingBottom: '120px' })}>
  <DiscoveryTagChips active={null} tags={discovery.tags} />
  <DiscoveryListSection initialHasMore={discovery.publications.hasMore} publications={discovery.publications.publications} />
</div>
