<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import DiscoverySectionHead from '../(site)/DiscoverySectionHead.svelte';
  import { currentSpaceSlug } from './current-space-slug';
  import { tagPath } from './paths';
  import PublicationListSection from './PublicationListSection.svelte';
  import TagChip from './TagChip.svelte';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';

  type Props = {
    tags: readonly { name: string; count: number }[];
    active: string;
    publications: readonly ({ id: string } & UsersiteApex_DiscoveryCard_publicationView$key)[];
    hasMore: boolean;
  };

  let { tags, active, publications, hasMore }: Props = $props();

  const slug = $derived(currentSpaceSlug());
  const count = $derived(tags.find((tag) => tag.name === active)?.count ?? publications.length);
</script>

<DiscoverySectionHead {count} title={`#${active}`} />

<div class={flex({ flexWrap: 'wrap', gap: '6px', marginBottom: '24px' })} aria-label="태그">
  {#each tags as tag (tag.name)}
    <TagChip name={tag.name} count={tag.count} current={tag.name === active} href={tagPath(slug, tag.name)} noscroll={false} size="lg" />
  {/each}
</div>

<div class={css({ minWidth: '0' })}>
  <PublicationListSection initialHasMore={hasMore} {publications} tagName={active} />
</div>
