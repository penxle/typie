<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { page } from '$app/state';
  import { graphql } from '$mearie';
  import DiscoveryCard from '../(site)/DiscoveryCard.svelte';

  type Props = {
    after: string | null;
    showCollection?: boolean;
    onLoaded: (result: { hasMore: boolean; lastId: string | null }) => void;
  };

  let { after, showCollection = true, onLoaded }: Props = $props();

  const query = createQuery(
    graphql(`
      query UsersiteSpace_PublicationListPage_Query($slug: String!, $after: ID) {
        spaceView(slug: $slug) {
          id

          publications(after: $after) {
            hasMore

            publications {
              id
              ...UsersiteApex_DiscoveryCard_publicationView
            }
          }
        }
      }
    `),
    () => ({ slug: page.params.slug ?? '', after }),
  );

  const result = $derived(query.data?.spaceView.publications);

  $effect(() => {
    if (!result) return;
    onLoaded({ hasMore: result.hasMore, lastId: result.publications.at(-1)?.id ?? null });
  });
</script>

{#if result}
  {#each result.publications as publication (publication.id)}
    <DiscoveryCard context="space" enter publicationView$key={publication} {showCollection} />
  {/each}
{/if}
