<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { infiniteScroll } from '@typie/ui/actions';
  import { Button, RingSpinner } from '@typie/ui/components';
  import { discoveryCardList } from './discovery-styles';
  import DiscoveryCard from './DiscoveryCard.svelte';
  import DiscoveryCardListPage from './DiscoveryCardListPage.svelte';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';
  import type { DiscoveryCardListSource } from './DiscoveryCardListPage.svelte';

  type FeedItem = {
    id: string;
  } & UsersiteApex_DiscoveryCard_publicationView$key;

  type Props = {
    publications: readonly FeedItem[];
    initialHasMore: boolean;
    source: DiscoveryCardListSource;
  };

  let { publications, initialHasMore, source }: Props = $props();

  let pages = $state<{ after: string }[]>([]);
  let hasMore = $state(false);
  let lastId = $state<string | null>(null);
  let pending = $state(false);

  const initialLastId = $derived(publications.at(-1)?.id ?? null);
  const initialHasMoreValue = $derived(initialHasMore);

  $effect(() => {
    hasMore = initialHasMoreValue;
    lastId = initialLastId;
  });

  const onLoaded = (result: { hasMore: boolean; lastId: string | null }) => {
    hasMore = result.hasMore;
    lastId = result.lastId ?? lastId;
    pending = false;
  };

  const loadMore = () => {
    const after = lastId;
    if (pending || !after || pages.some((page) => page.after === after)) return;
    pending = true;
    pages = [...pages, { after }];
  };
</script>

{#if publications.length > 0}
  <div class={css(discoveryCardList, { marginTop: '20px' })}>
    {#each publications as publication (publication.id)}
      <DiscoveryCard publicationView$key={publication} />
    {/each}

    {#each pages as page (page.after)}
      <DiscoveryCardListPage after={page.after} {onLoaded} {source} />
    {/each}
  </div>

  {#if hasMore && lastId}
    <div
      class={flex({ justifyContent: 'center', minHeight: '20px', marginTop: '24px', lgDown: { display: 'none' } })}
      use:infiniteScroll={{ onLoadMore: loadMore, enabled: !pending, rootMargin: '0px 0px 600px 0px' }}
    >
      {#if pending}
        <RingSpinner style={css.raw({ size: '20px', color: 'text.muted' })} />
      {/if}
    </div>

    <div class={flex({ justifyContent: 'center', marginTop: '24px', lg: { display: 'none' } })}>
      <Button loading={pending} onclick={loadMore} size="sm" variant="secondary">더 보기</Button>
    </div>
  {/if}
{:else}
  <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', paddingY: '80px' })}>
    <p class={css({ fontSize: '14px', color: 'text.hint' })}>아직 발행된 글이 없어요</p>
  </div>
{/if}
