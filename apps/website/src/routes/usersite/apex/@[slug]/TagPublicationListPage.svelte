<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { page } from '$app/state';
  import { graphql } from '$mearie';
  import DiscoveryCard from '../(site)/DiscoveryCard.svelte';

  type Props = {
    after: string | null;
    tagName: string;
    showCollection?: boolean;
    onLoaded: (result: { hasMore: boolean; lastId: string | null }) => void;
  };

  let { after, tagName, showCollection = true, onLoaded }: Props = $props();

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
                ...UsersiteApex_DiscoveryCard_publicationView
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
    <DiscoveryCard context="space" enter publicationView$key={publication} {showCollection} />
  {/each}
{/if}
