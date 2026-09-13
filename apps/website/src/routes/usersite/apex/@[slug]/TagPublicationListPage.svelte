<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { page } from '$app/state';
  import { graphql } from '$mearie';
  import PublicationListItem from './PublicationListItem.svelte';
  import type { SpaceDateDisplay } from '$mearie';

  type Props = {
    after: string | null;
    tagName: string;
    dateDisplay: SpaceDateDisplay;
    showCollection?: boolean;
    onLoaded: (result: { hasMore: boolean; lastId: string | null }) => void;
  };

  let { after, tagName, dateDisplay, showCollection = true, onLoaded }: Props = $props();

  const query = createQuery(
    graphql(`
      query UsersiteSpace_TagPublicationListPage_Query($slug: String!, $name: String!, $after: ID) {
        spaceView(slug: $slug) {
          id

          tag(name: $name) {
            name

            publications(after: $after) {
              hasMore

              publications {
                id
                ...UsersiteSpace_PublicationListItem_publicationView
              }
            }
          }
        }
      }
    `),
    () => ({ slug: page.params.slug ?? '', name: tagName, after }),
  );

  const result = $derived(query.data?.spaceView.tag.publications);

  $effect(() => {
    if (!result) return;
    onLoaded({ hasMore: result.hasMore, lastId: result.publications.at(-1)?.id ?? null });
  });
</script>

{#if result}
  {#each result.publications as publication (publication.id)}
    <div in:fly={{ y: 6, duration: 220, easing: cubicOut }}>
      <PublicationListItem {dateDisplay} publicationView$key={publication} {showCollection} />
    </div>
  {/each}
{/if}
