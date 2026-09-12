<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { groupConsecutiveBySpace } from '$lib/discovery/feed-grouping';
  import PublicationListItem from '../wildcard/PublicationListItem.svelte';
  import DiscoveryListPage from './DiscoveryListPage.svelte';
  import DiscoveryTagListPage from './DiscoveryTagListPage.svelte';
  import FeedMoreRow from './FeedMoreRow.svelte';
  import type { UsersiteWildcard_PublicationListItem_publicationView$key } from '$mearie';

  type FeedItem = {
    id: string;
    space: { id: string; name: string; url: string };
  } & UsersiteWildcard_PublicationListItem_publicationView$key;

  type Props = {
    publications: readonly FeedItem[];
    initialHasMore?: boolean;
    tagName?: string | null;
  };

  let { publications, initialHasMore = false, tagName = null }: Props = $props();

  let cursors = $state<string[]>([]);
  let hasMore = $state(false);
  let lastId = $state<string | null>(null);
  let pending = $state(false);

  const initialLastId = $derived(publications.at(-1)?.id ?? null);
  const initialHasMoreValue = $derived(initialHasMore);

  $effect(() => {
    hasMore = initialHasMoreValue;
    lastId = initialLastId;
  });

  const groups = $derived(groupConsecutiveBySpace(publications));

  const onLoaded = (result: { hasMore: boolean; lastId: string | null }) => {
    hasMore = result.hasMore;
    lastId = result.lastId ?? lastId;
    pending = false;
  };
</script>

{#if publications.length > 0}
  <section>
    <div class={flex({ flexDirection: 'column' })}>
      {#each groups as group, index (group.lead.id)}
        <PublicationListItem dateDisplay="PUBLISHED_AT" first={index === 0} publicationView$key={group.lead} showSpace />
        {#if group.more > 0}
          <FeedMoreRow count={group.more} href={group.lead.space.url} />
        {/if}
      {/each}

      {#each cursors as after (after)}
        {#if tagName === null}
          <DiscoveryListPage {after} {onLoaded} />
        {:else}
          <DiscoveryTagListPage {after} {onLoaded} {tagName} />
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
    <p class={css({ fontSize: '14px', color: 'text.hint' })}>아직 발행된 글이 없어요</p>
  </div>
{/if}
