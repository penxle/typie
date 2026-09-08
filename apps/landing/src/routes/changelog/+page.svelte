<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import Page from '$lib/components/Page.svelte';
  import { COPY } from './changelog';
  import ChangelogFeed from './ChangelogFeed.svelte';
  import { Feed } from './feed.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };

  let { data }: Props = $props();

  const feed = new Feed(data.initial);
</script>

<Helmet description={COPY.description} title={COPY.pageTitle} />

<Page sub={COPY.sub}>
  {#snippet title()}
    {COPY.title[0]}
    <br />
    <span class={css({ color: 'text.muted' })}>{COPY.title[1]}</span>
  {/snippet}
  <ChangelogFeed {feed} />
</Page>
