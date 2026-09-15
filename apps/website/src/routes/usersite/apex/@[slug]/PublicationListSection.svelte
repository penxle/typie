<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { infiniteScroll } from '@typie/ui/actions';
  import { Button, RingSpinner } from '@typie/ui/components';
  import { discoveryCardList } from '../(site)/discovery-styles';
  import DiscoveryCard from '../(site)/DiscoveryCard.svelte';
  import TagPublicationListPage from './TagPublicationListPage.svelte';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';

  type Props = {
    publications: readonly ({ id: string } & UsersiteApex_DiscoveryCard_publicationView$key)[];
    initialHasMore?: boolean;
    tagName: string;
    showFolder?: boolean;
  };

  let { publications, initialHasMore = false, tagName, showFolder = true }: Props = $props();

  let cursors = $state<string[]>([]);
  let hasMore = $state(false);
  let lastId = $state<string | null>(null);
  let pending = $state(false);

  $effect(() => {
    hasMore = initialHasMore;
    lastId = publications.at(-1)?.id ?? null;
  });

  const onLoaded = (result: { hasMore: boolean; lastId: string | null }) => {
    hasMore = result.hasMore;
    lastId = result.lastId ?? lastId;
    pending = false;
  };

  const loadMore = () => {
    const after = lastId;
    if (pending || !after || cursors.includes(after)) return;
    pending = true;
    cursors = [...cursors, after];
  };
</script>

{#if publications.length > 0}
  <section>
    <div class={css(discoveryCardList)}>
      {#each publications as publication (publication.id)}
        <DiscoveryCard context="space" publicationView$key={publication} {showFolder} />
      {/each}

      {#each cursors as after (after)}
        <TagPublicationListPage {after} {onLoaded} {showFolder} {tagName} />
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
  </section>
{:else}
  <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', paddingY: '80px' })}>
    <p class={css({ fontSize: '14px', color: 'text.hint' })}>아직 발행한 글이 없어요</p>
  </div>
{/if}
