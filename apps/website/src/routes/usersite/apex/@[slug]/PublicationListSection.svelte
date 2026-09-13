<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { discoveryCardList } from '../(site)/discovery-styles';
  import DiscoveryCard from '../(site)/DiscoveryCard.svelte';
  import PublicationListPage from './PublicationListPage.svelte';
  import TagPublicationListPage from './TagPublicationListPage.svelte';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';

  type Props = {
    publications: readonly ({ id: string } & UsersiteApex_DiscoveryCard_publicationView$key)[];
    initialHasMore?: boolean;
    tagName?: string | null;
    showCollection?: boolean;
  };

  let { publications, initialHasMore = false, tagName = null, showCollection = true }: Props = $props();

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
</script>

{#if publications.length > 0}
  <section>
    <div class={css(discoveryCardList)}>
      {#each publications as publication (publication.id)}
        <DiscoveryCard context="space" publicationView$key={publication} {showCollection} />
      {/each}

      {#each cursors as after (after)}
        {#if tagName === null}
          <PublicationListPage {after} {onLoaded} {showCollection} />
        {:else}
          <TagPublicationListPage {after} {onLoaded} {showCollection} {tagName} />
        {/if}
      {/each}
    </div>

    {#if hasMore && lastId}
      <div class={flex({ justifyContent: 'center', marginTop: '24px' })}>
        <Button
          loading={pending}
          onclick={() => {
            if (lastId && !cursors.includes(lastId)) {
              pending = true;
              cursors = [...cursors, lastId];
            }
          }}
          size="sm"
          variant="secondary"
        >
          더 보기
        </Button>
      </div>
    {/if}
  </section>
{:else}
  <div class={flex({ flexDirection: 'column', alignItems: 'center', justifyContent: 'center', paddingY: '80px' })}>
    <p class={css({ fontSize: '14px', color: 'text.hint' })}>아직 발행한 글이 없어요</p>
  </div>
{/if}
