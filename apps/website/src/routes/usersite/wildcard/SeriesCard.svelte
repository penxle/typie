<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { seriesPath } from './paths';
  import type { UsersiteWildcard_SeriesCard_collectionView$key } from '$mearie';

  type Props = {
    collectionView$key: UsersiteWildcard_SeriesCard_collectionView$key;
  };

  let { collectionView$key }: Props = $props();

  const collection = createFragment(
    graphql(`
      fragment UsersiteWildcard_SeriesCard_collectionView on CollectionView {
        id
        name
        description

        cover {
          id
          ...Img_image
        }

        publications {
          id
          title
          publishedAt
        }
      }
    `),
    () => collectionView$key,
  );

  const latest = $derived(collection.data.publications.toSorted((a, b) => b.publishedAt.localeCompare(a.publishedAt))[0] ?? null);
</script>

<a
  class={css({
    display: 'flex',
    gap: '14px',
    width: 'full',
    paddingY: '16px',
    borderBottomWidth: '1px',
    borderColor: 'border.hairline',
    textAlign: 'left',
    _hover: { '& .series-name': { color: 'text.muted' } },
  })}
  data-sveltekit-noscroll
  href={seriesPath(collection.data.id)}
>
  <div class={css({ flexShrink: '0', size: '56px', borderRadius: '10px', backgroundColor: 'surface.canvas', overflow: 'hidden' })}>
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
    <div class={css({ fontSize: '16px', fontWeight: 'semibold', transition: 'colors' })}>
      <span class="series-name">{collection.data.name}</span>
      <span
        class={css({ marginLeft: '8px', fontSize: '12px', fontWeight: 'normal', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}
      >
        글 {collection.data.publications.length}개
      </span>
    </div>
    {#if collection.data.description}
      <p class={css({ marginTop: '3px', fontSize: '13px', lineHeight: '[1.5]', color: 'text.muted', lineClamp: '2' })}>
        {collection.data.description}
      </p>
    {/if}
    {#if latest}
      <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums', lineClamp: '1' })}>
        최근 글 <b class={css({ fontWeight: 'medium', color: 'text.muted' })}>{latest.title}</b>
        · {dayjs(latest.publishedAt).format('YYYY. M. D.')}
      </p>
    {/if}
  </div>

  <Icon style={css.raw({ flexShrink: '0', alignSelf: 'center', color: 'text.hint' })} icon={ChevronRightIcon} size={16} />
</a>
