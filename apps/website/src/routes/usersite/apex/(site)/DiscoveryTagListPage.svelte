<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { groupConsecutiveBySpace } from '$lib/discovery/feed-grouping';
  import { graphql } from '$mearie';
  import PublicationListItem from '../@[slug]/PublicationListItem.svelte';
  import FeedMoreRow from './FeedMoreRow.svelte';

  type Props = {
    after: string | null;
    tagName: string;
    onLoaded: (result: { hasMore: boolean; lastId: string | null }) => void;
  };

  let { after, tagName, onLoaded }: Props = $props();

  const query = createQuery(
    graphql(`
      query UsersiteApex_DiscoveryTagListPage_Query($name: String!, $after: ID) {
        discovery {
          tag(name: $name) {
            name

            publications(after: $after) {
              hasMore

              publications {
                id

                space {
                  id
                  name
                  url
                }

                ...UsersiteSpace_PublicationListItem_publicationView
              }
            }
          }
        }
      }
    `),
    () => ({ name: tagName, after }),
  );

  const result = $derived(query.data?.discovery.tag?.publications);
  const groups = $derived(result ? groupConsecutiveBySpace(result.publications) : []);

  $effect(() => {
    if (!result) return;
    onLoaded({ hasMore: result.hasMore, lastId: result.publications.at(-1)?.id ?? null });
  });
</script>

{#each groups as group (group.lead.id)}
  <div in:fly={{ y: 6, duration: 220, easing: cubicOut }}>
    <PublicationListItem dateDisplay="PUBLISHED_AT" publicationView$key={group.lead} showSpace />
    {#if group.more > 0}
      <FeedMoreRow count={group.more} href={group.lead.space.url} />
    {/if}
  </div>
{/each}
