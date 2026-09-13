<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { latestOfSpaceRuns } from '$lib/discovery/feed-grouping';
  import { graphql } from '$mearie';
  import DiscoveryCard from './DiscoveryCard.svelte';

  type Props = {
    after: string;
    previousSpaceId: string | null;
    tagName: string | null;
    onLoaded: (result: { hasMore: boolean; lastId: string | null; lastSpaceId: string | null }) => void;
  };

  let { after, previousSpaceId, tagName, onLoaded }: Props = $props();

  const feedQuery = createQuery(
    graphql(`
      query UsersiteApex_DiscoveryCardListPage_Query($after: ID) {
        discovery {
          publications(after: $after) {
            hasMore

            publications {
              id

              space {
                id
              }

              ...UsersiteApex_DiscoveryCard_publicationView
            }
          }
        }
      }
    `),
    () => ({ after }),
    () => ({ skip: tagName !== null }),
  );

  const tagQuery = createQuery(
    graphql(`
      query UsersiteApex_DiscoveryCardListTagPage_Query($name: String!, $after: ID) {
        discovery {
          tag(name: $name) {
            name

            publications(after: $after) {
              hasMore

              publications {
                id

                space {
                  id
                }

                ...UsersiteApex_DiscoveryCard_publicationView
              }
            }
          }
        }
      }
    `),
    () => ({ name: tagName ?? '', after }),
    () => ({ skip: tagName === null }),
  );

  const result = $derived(tagName === null ? feedQuery.data?.discovery.publications : tagQuery.data?.discovery.tag?.publications);
  const leads = $derived(result ? latestOfSpaceRuns(result.publications, previousSpaceId) : []);

  $effect(() => {
    if (!result) return;
    onLoaded({
      hasMore: result.hasMore,
      lastId: result.publications.at(-1)?.id ?? null,
      lastSpaceId: result.publications.at(-1)?.space.id ?? null,
    });
  });
</script>

{#each leads as publication (publication.id)}
  <DiscoveryCard enter publicationView$key={publication} />
{/each}
