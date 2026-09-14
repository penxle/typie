<script lang="ts" module>
  export type DiscoveryCardListSource = { kind: 'feed' } | { kind: 'publications' } | { kind: 'tag'; name: string };
</script>

<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { graphql } from '$mearie';
  import DiscoveryCard from './DiscoveryCard.svelte';

  type Props = {
    after: string;
    source: DiscoveryCardListSource;
    onLoaded: (result: { hasMore: boolean; lastId: string | null }) => void;
  };

  let { after, source, onLoaded }: Props = $props();

  const feedQuery = createQuery(
    graphql(`
      query UsersiteApex_DiscoveryCardListFeedPage_Query($after: ID) {
        discovery {
          feed(after: $after) {
            hasMore

            publications {
              id
              ...UsersiteApex_DiscoveryCard_publicationView
            }
          }
        }
      }
    `),
    () => ({ after }),
    () => ({ skip: source.kind !== 'feed' }),
  );

  const publicationsQuery = createQuery(
    graphql(`
      query UsersiteApex_DiscoveryCardListPage_Query($after: ID) {
        discovery {
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
    () => ({ after }),
    () => ({ skip: source.kind !== 'publications' }),
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
                ...UsersiteApex_DiscoveryCard_publicationView
              }
            }
          }
        }
      }
    `),
    () => ({ name: source.kind === 'tag' ? source.name : '', after }),
    () => ({ skip: source.kind !== 'tag' }),
  );

  const result = $derived.by(() => {
    switch (source.kind) {
      case 'feed': {
        return feedQuery.data?.discovery.feed;
      }
      case 'publications': {
        return publicationsQuery.data?.discovery.publications;
      }
      case 'tag': {
        return tagQuery.data?.discovery.tag?.publications;
      }
    }
  });

  $effect(() => {
    if (!result) return;
    onLoaded({
      hasMore: result.hasMore,
      lastId: result.publications.at(-1)?.id ?? null,
    });
  });
</script>

{#if result}
  {#each result.publications as publication (publication.id)}
    <DiscoveryCard enter publicationView$key={publication} />
  {/each}
{/if}
