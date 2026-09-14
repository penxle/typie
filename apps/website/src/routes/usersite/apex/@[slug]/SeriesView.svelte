<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import PublicationListSection from './PublicationListSection.svelte';
  import type { SpaceDateDisplay, UsersiteSpace_SeriesView_collectionView$key } from '$mearie';

  type Props = {
    collectionView$key: UsersiteSpace_SeriesView_collectionView$key;
    dateDisplay: SpaceDateDisplay;
  };

  let { collectionView$key, dateDisplay }: Props = $props();

  const collection = createFragment(
    graphql(`
      fragment UsersiteSpace_SeriesView_collectionView on CollectionView {
        id
        name
        description

        cover {
          id
          ...Img_image
        }

        publications {
          id
          ...UsersiteSpace_PublicationListItem_publicationView
        }
      }
    `),
    () => collectionView$key,
  );
</script>

<div
  class={flex({
    alignItems: 'flex-start',
    gap: '14px',
    paddingTop: '16px',
    paddingBottom: '14px',
    borderBottomWidth: '1px',
    borderColor: 'border.hairline',
  })}
>
  <div
    class={css({
      flexShrink: '0',
      size: '56px',
      borderRadius: '10px',
      backgroundColor: 'surface.canvas',
      boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
      overflow: 'hidden',
    })}
  >
    {#if collection.data.cover}
      <Img
        style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
        alt={collection.data.name}
        image$key={collection.data.cover}
        size={128}
      />
    {/if}
  </div>

  <div class={css({ flex: '1', minWidth: '0' })}>
    <h2 class={css({ fontSize: '18px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>{collection.data.name}</h2>
    {#if collection.data.description}
      <p class={css({ marginTop: '4px', fontSize: '14px', lineHeight: '[1.5]', color: 'text.muted' })}>{collection.data.description}</p>
    {/if}
    <div
      class={flex({
        alignItems: 'center',
        gap: '8px',
        marginTop: '8px',
        fontSize: '12px',
        color: 'text.hint',
        fontVariantNumeric: 'tabular-nums',
      })}
    >
      <span>글 {collection.data.publications.length}개</span>
    </div>
  </div>
</div>

<PublicationListSection {dateDisplay} publications={collection.data.publications} showCollection={false} />
