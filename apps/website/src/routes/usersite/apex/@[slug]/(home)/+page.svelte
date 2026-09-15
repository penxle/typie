<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { hydrateQuery } from '$lib/graphql';
  import SiteEntries from '../SiteEntries.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
  const site = $derived(query.data.siteView);
</script>

<Helmet description={site.description ?? site.name} title={site.name} />

{#if site.entries.length > 0}
  <SiteEntries entries={site.entries} />
{:else}
  <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', paddingY: '80px' })}>
    <p class={css({ fontSize: '14px', color: 'text.hint' })}>아직 발행한 글이 없어요</p>
  </div>
{/if}
