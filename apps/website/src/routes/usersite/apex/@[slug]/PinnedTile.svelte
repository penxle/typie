<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import TextIcon from '~icons/lucide/text';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { currentSpaceSlug } from './current-space-slug';
  import { publicationPath } from './paths';
  import type { SpaceDateDisplay, UsersiteSpace_PinnedTile_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpace_PinnedTile_publicationView$key;
    dateDisplay: SpaceDateDisplay;
  };

  let { publicationView$key, dateDisplay }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpace_PinnedTile_publicationView on PublicationView {
        id
        title
        publishedAt
        updatedAt

        thumbnail {
          id
          ...Img_image
        }

        collection {
          id
          name
        }
      }
    `),
    () => publicationView$key,
  );

  const slug = $derived(currentSpaceSlug());
  const sub = $derived.by(() => {
    if (publication.data.collection) return publication.data.collection.name;
    if (dateDisplay === 'NONE') return null;
    return dayjs(dateDisplay === 'PUBLISHED_AT' ? publication.data.publishedAt : publication.data.updatedAt).format('YYYY. M. D.');
  });
</script>

<a
  class={css({
    display: 'flex',
    flexDirection: { base: 'row', lg: 'column' },
    alignItems: { base: 'center', lg: 'stretch' },
    gap: { base: '12px', lg: '10px' },
    paddingX: { base: '12px', lg: '14px' },
    paddingY: { base: '10px', lg: '14px' },
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '12px',
    transition: 'common',
    _hover: { borderColor: 'border.default', '& img': { transform: 'scale(1.03)' } },
  })}
  href={publicationPath(slug, publication.data.id)}
>
  <div
    class={css({
      flexShrink: '0',
      width: { base: '48px', lg: 'full' },
      height: { base: '48px', lg: 'auto' },
      aspectRatio: { lg: '[3/2]' },
      borderRadius: { base: '6px', lg: '8px' },
      backgroundColor: 'surface.canvas',
      overflow: 'hidden',
      isolation: 'isolate',
    })}
  >
    {#if publication.data.thumbnail}
      <Img
        style={css.raw({
          width: 'full',
          height: 'full',
          objectFit: 'cover',
          transition: '[transform 240ms cubic-bezier(0.32, 0.72, 0, 1)]',
        })}
        alt={publication.data.title}
        image$key={publication.data.thumbnail}
        size={512}
      />
    {:else}
      <div
        class={css({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          width: 'full',
          height: 'full',
          backgroundColor: 'surface.inset',
          color: 'text.hint',
        })}
      >
        <Icon icon={TextIcon} size={16} />
      </div>
    {/if}
  </div>

  <div class={css({ minWidth: '0' })}>
    <div class={css({ fontSize: { base: '14px', lg: '15px' }, fontWeight: 'semibold', lineHeight: '[1.4]', lineClamp: '2' })}>
      {publication.data.title}
    </div>
    {#if sub}
      <div class={css({ marginTop: '2px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums', lineClamp: '1' })}>
        {sub}
      </div>
    {/if}
  </div>
</a>
