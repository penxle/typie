<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { COPY } from './changelog';
  import type { Feed } from './feed.svelte';

  type Props = { feed: Feed };

  let { feed }: Props = $props();

  const hostClass = css({ display: 'grid', justifyItems: 'center', paddingY: '40px', fontSize: '13px', color: 'text.hint' });
</script>

{#if feed.failed}
  <div class={hostClass}>{COPY.failed}</div>
{:else if feed.hasMore}
  {#key feed.nextPage}
    <div class={hostClass} {@attach feed.observe}></div>
  {/key}
{/if}
