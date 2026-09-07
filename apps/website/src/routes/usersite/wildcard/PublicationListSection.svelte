<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import PublicationListItem from './PublicationListItem.svelte';
  import PublicationListPage from './PublicationListPage.svelte';
  import TagPublicationListPage from './TagPublicationListPage.svelte';
  import type { SpaceDateDisplay, UsersiteWildcard_PublicationListItem_publicationView$key } from '$mearie';

  type Props = {
    publications: readonly ({ id: string } & UsersiteWildcard_PublicationListItem_publicationView$key)[];
    dateDisplay: SpaceDateDisplay;
    initialHasMore?: boolean;
    tagName?: string | null;
    showCollection?: boolean;
  };

  let { publications, dateDisplay, initialHasMore = false, tagName = null, showCollection = true }: Props = $props();

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
    <div class={flex({ flexDirection: 'column' })}>
      {#each publications as publication, index (publication.id)}
        <PublicationListItem {dateDisplay} first={index === 0} publicationView$key={publication} {showCollection} />
      {/each}

      {#each cursors as after (after)}
        {#if tagName === null}
          <PublicationListPage {after} {dateDisplay} {onLoaded} {showCollection} />
        {:else}
          <TagPublicationListPage {after} {dateDisplay} {onLoaded} {showCollection} {tagName} />
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
