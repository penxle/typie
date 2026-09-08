<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Helmet } from '@typie/ui/components';
  import Page from '$lib/landing/components/Page.svelte';
  import { detectedPlatform } from '$lib/landing/platform.svelte';
  import { COPY, primaryFor } from './download';
  import DownloadGroups from './DownloadGroups.svelte';
  import PrimaryDownload from './PrimaryDownload.svelte';

  const detected = detectedPlatform();

  const primary = $derived(primaryFor(detected.current));

  const actionClass = css({ display: 'grid', justifyItems: 'center', marginTop: { base: '32px', md: '40px' }, minHeight: '[88px]' });
  const groupsClass = css({ marginTop: { base: '64px', lg: '96px' } });
</script>

<Helmet description={COPY.description} title={COPY.pageTitle} />

<Page sub={COPY.sub}>
  {#snippet title()}{COPY.title}{/snippet}
  {#snippet hero()}
    <div class={actionClass}>
      {#if primary}
        <PrimaryDownload {primary} />
      {/if}
    </div>
  {/snippet}
  <div class={groupsClass}><DownloadGroups /></div>
</Page>
