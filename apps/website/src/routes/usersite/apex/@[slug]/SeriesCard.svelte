<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { currentSpaceSlug } from './current-space-slug';
  import { seriesPath } from './paths';
  import type { UsersiteSpace_SeriesCard_collectionView$key } from '$mearie';

  type Props = {
    collectionView$key: UsersiteSpace_SeriesCard_collectionView$key;
  };

  let { collectionView$key }: Props = $props();

  const collection = createFragment(
    graphql(`
      fragment UsersiteSpace_SeriesCard_collectionView on CollectionView {
        id
        permalink
        name
        description

        cover {
          id
          ...Img_image
        }

        publications {
          id
        }
      }
    `),
    () => collectionView$key,
  );

  const slug = $derived(currentSpaceSlug());

  const meta = css.raw({
    marginTop: '6px',
    fontSize: '12px',
    color: 'text.hint',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
  });
</script>

<a
  class={css({
    display: 'flex',
    gap: '16px',
    minWidth: '0',
    padding: '16px',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '12px',
    transition: 'common',
    _hover: { borderColor: 'border.default' },
  })}
  href={seriesPath(slug, collection.data.permalink)}
>
  <div
    style:background-color={collection.data.cover ? undefined : titlePageColors(collection.data.name).base}
    class={css({
      flexShrink: '0',
      width: '64px',
      height: '96px',
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
    <div class={css({ fontSize: '16px', fontWeight: 'semibold', lineHeight: '[1.4]' })}>{collection.data.name}</div>
    {#if collection.data.description}
      <p class={css({ marginTop: '4px', fontSize: '13px', lineHeight: '[1.5]', color: 'text.muted', lineClamp: '2' })}>
        {collection.data.description}
      </p>
    {/if}
    <p class={css(meta)}>글 {collection.data.publications.length}개</p>
  </div>
</a>
