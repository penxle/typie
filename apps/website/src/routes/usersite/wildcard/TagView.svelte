<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tagPath } from './paths';
  import PublicationListSection from './PublicationListSection.svelte';
  import TagChip from './TagChip.svelte';
  import type { SpaceDateDisplay, UsersiteWildcard_PublicationListItem_publicationView$key } from '$mearie';

  type Props = {
    tags: readonly { name: string; count: number }[];
    active: string;
    dateDisplay: SpaceDateDisplay;
    publications: readonly ({ id: string } & UsersiteWildcard_PublicationListItem_publicationView$key)[];
    hasMore: boolean;
  };

  let { tags, active, dateDisplay, publications, hasMore }: Props = $props();

  const count = $derived(tags.find((tag) => tag.name === active)?.count ?? publications.length);
</script>

<div
  class={flex({ display: { base: 'flex', lg: 'none' }, flexWrap: 'wrap', gap: '8px', paddingTop: '14px', paddingBottom: '8px' })}
  aria-label="태그"
>
  {#each tags as tag (tag.name)}
    <TagChip name={tag.name} count={tag.count} current={tag.name === active} href={tagPath(tag.name)} size="lg" />
  {/each}
</div>

<div
  class={css({
    display: { base: 'none', lg: 'block' },
    paddingTop: '16px',
    paddingBottom: '14px',
    borderBottomWidth: '1px',
    borderColor: 'border.hairline',
  })}
>
  <h2 class={css({ fontSize: '18px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>#{active}</h2>
  <div class={css({ marginTop: '8px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>글 {count}개</div>
</div>

<PublicationListSection {dateDisplay} initialHasMore={hasMore} {publications} tagName={active} />
