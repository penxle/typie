<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { latestOfSpaceRuns } from '$lib/discovery/feed-grouping';
  import { discoveryCardList } from './discovery-styles';
  import DiscoveryCard from './DiscoveryCard.svelte';
  import DiscoveryCardListPage from './DiscoveryCardListPage.svelte';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';

  type FeedItem = {
    id: string;
    space: { id: string };
  } & UsersiteApex_DiscoveryCard_publicationView$key;

  type Props = {
    publications: readonly FeedItem[];
    initialHasMore: boolean;
    tagName?: string | null;
  };

  let { publications, initialHasMore, tagName = null }: Props = $props();

  let pages = $state<{ after: string; previousSpaceId: string | null }[]>([]);
  let hasMore = $state(false);
  let lastId = $state<string | null>(null);
  let lastSpaceId = $state<string | null>(null);
  let pending = $state(false);

  const initialLastId = $derived(publications.at(-1)?.id ?? null);
  const initialLastSpaceId = $derived(publications.at(-1)?.space.id ?? null);
  const initialHasMoreValue = $derived(initialHasMore);

  $effect(() => {
    hasMore = initialHasMoreValue;
    lastId = initialLastId;
    lastSpaceId = initialLastSpaceId;
  });

  const leads = $derived(latestOfSpaceRuns(publications));

  const onLoaded = (result: { hasMore: boolean; lastId: string | null; lastSpaceId: string | null }) => {
    hasMore = result.hasMore;
    lastId = result.lastId ?? lastId;
    lastSpaceId = result.lastSpaceId ?? lastSpaceId;
    pending = false;
  };
</script>

{#if publications.length > 0}
  <div class={css(discoveryCardList, { marginTop: '20px' })}>
    {#each leads as publication (publication.id)}
      <DiscoveryCard publicationView$key={publication} />
    {/each}

    {#each pages as page (page.after)}
      <DiscoveryCardListPage after={page.after} {onLoaded} previousSpaceId={page.previousSpaceId} {tagName} />
    {/each}
  </div>

  {#if hasMore && lastId}
    <div class={flex({ justifyContent: 'center', marginTop: '24px' })}>
      <Button
        loading={pending}
        onclick={() => {
          const after = lastId;
          if (after && pages.every((page) => page.after !== after)) {
            pending = true;
            pages = [...pages, { after, previousSpaceId: lastSpaceId }];
          }
        }}
        size="sm"
        variant="secondary"
      >
        더 보기
      </Button>
    </div>
  {/if}
{:else}
  <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', paddingY: '80px' })}>
    <p class={css({ fontSize: '14px', color: 'text.hint' })}>아직 발행된 글이 없어요</p>
  </div>
{/if}
