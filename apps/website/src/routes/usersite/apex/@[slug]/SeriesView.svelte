<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import PublicationListSection from './PublicationListSection.svelte';
  import type { UsersiteSpace_SeriesView_collectionView$key } from '$mearie';

  type Props = {
    collectionView$key: UsersiteSpace_SeriesView_collectionView$key;
  };

  let { collectionView$key }: Props = $props();

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
          ...UsersiteApex_DiscoveryCard_publicationView
        }
      }
    `),
    () => collectionView$key,
  );
</script>

<div class={flex({ alignItems: 'flex-start', gap: '16px', marginBottom: '20px' })}>
  <div
    style:background-color={collection.data.cover ? undefined : titlePageColors(collection.data.name).base}
    class={css({
      flexShrink: '0',
      width: '56px',
      height: '84px',
      borderRadius: '6px',
      backgroundColor: 'surface.inset',
      boxShadow: '[inset 0 0 0 1px token(colors.border.hairline)]',
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
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>{collection.data.name}</h1>
    {#if collection.data.description}
      <p class={css({ marginTop: '6px', maxWidth: '560px', fontSize: '14px', lineHeight: '[1.6]', color: 'text.muted' })}>
        {collection.data.description}
      </p>
    {/if}
    <div class={css({ marginTop: '6px', fontSize: '13px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
      글 {collection.data.publications.length}개
    </div>
  </div>
</div>

<PublicationListSection publications={collection.data.publications} showCollection={false} />
