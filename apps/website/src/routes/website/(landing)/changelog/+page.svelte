<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import Page from '$lib/landing/components/Page.svelte';
  import Shell from '$lib/landing/components/Shell.svelte';
  import { COPY } from './changelog';
  import ChangelogFeed from './ChangelogFeed.svelte';
  import { Feed } from './feed.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };

  let { data }: Props = $props();

  const feed = new Feed(data.initial);

  const previewClass = css({
    width: 'full',
    maxWidth: '[1040px]',
    marginX: 'auto',
    paddingX: { base: '20px', lg: '40px' },
    paddingTop: { base: '128px', lg: '176px' },
    paddingBottom: { base: '112px', lg: '160px' },
  });
</script>

<Helmet description={COPY.description} title={COPY.pageTitle} />

{#if data.preview}
  <Shell>
    <main class={previewClass}>
      <ChangelogFeed {feed} />
    </main>
  </Shell>
{:else}
  <Page sub={COPY.sub}>
    {#snippet title()}
      {COPY.title[0]}
      <br />
      <span class={css({ color: 'text.muted' })}>{COPY.title[1]}</span>
    {/snippet}
    <ChangelogFeed {feed} />
  </Page>
{/if}
