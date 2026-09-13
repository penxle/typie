<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { graphql } from '$mearie';
  import { discoveryCardList } from '../../../(site)/discovery-styles';
  import DiscoveryCard from '../../../(site)/DiscoveryCard.svelte';
  import DiscoverySectionHead from '../../../(site)/DiscoverySectionHead.svelte';
  import { currentSpaceSlug } from '../../current-space-slug';
  import { spaceHomePath } from '../../paths';
  import type { UsersiteSpacePublicationPage_PublicationRecent_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpacePublicationPage_PublicationRecent_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpacePublicationPage_PublicationRecent_publicationView on PublicationView {
        id

        nextInCollection {
          id
        }

        space {
          id

          publications(first: 6) {
            publications {
              id
              ...UsersiteApex_DiscoveryCard_publicationView
            }
          }
        }
      }
    `),
    () => publicationView$key,
  );

  const slug = $derived(currentSpaceSlug());
  const items = $derived(
    publication.data.space.publications.publications
      .filter((item) => item.id !== publication.data.id && item.id !== publication.data.nextInCollection?.id)
      .slice(0, 4),
  );
</script>

{#if items.length > 0}
  <section>
    <DiscoverySectionHead moreHref={spaceHomePath(slug)} title="이 스페이스의 글" />
    <div class={css(discoveryCardList)}>
      {#each items as item (item.id)}
        <DiscoveryCard context="space" publicationView$key={item} />
      {/each}
    </div>
  </section>
{/if}
