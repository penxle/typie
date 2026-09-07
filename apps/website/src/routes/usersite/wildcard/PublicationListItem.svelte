<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import type { SpaceDateDisplay, UsersiteWildcard_PublicationListItem_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteWildcard_PublicationListItem_publicationView$key;
    dateDisplay: SpaceDateDisplay;
    first?: boolean;
    showCollection?: boolean;
  };

  let { publicationView$key, dateDisplay, first = false, showCollection = true }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteWildcard_PublicationListItem_publicationView on PublicationView {
        id
        title
        subtitle
        excerpt
        hasPassword
        passwordUnlocked
        publishedAt
        updatedAt
        tags

        collection {
          id
          name
        }

        thumbnail {
          id
          ...Img_image
        }
      }
    `),
    () => publicationView$key,
  );

  const date = $derived(
    dateDisplay === 'NONE'
      ? null
      : dayjs(dateDisplay === 'PUBLISHED_AT' ? publication.data.publishedAt : publication.data.updatedAt).format('YYYY. M. D.'),
  );
  const collectionName = $derived(showCollection ? (publication.data.collection?.name ?? null) : null);
  const hasMeta = $derived(date !== null || collectionName !== null || publication.data.tags.length > 0);
</script>

<a
  class={flex({
    gap: { base: '20px', md: '24px' },
    paddingY: '20px',
    borderTopWidth: first ? '0' : '1px',
    borderColor: 'border.hairline',
    cursor: 'pointer',
    _hover: { '& .publication-title': { color: 'text.muted' }, '& .publication-thumbnail img': { transform: 'scale(1.03)' } },
  })}
  href={`/p/${publication.data.id}`}
>
  <div class={css({ flex: '1', minWidth: '0' })}>
    <div class={flex({ alignItems: 'center', gap: '6px' })}>
      {#if publication.data.hasPassword}
        <Icon
          style={css.raw({ flexShrink: '0', color: 'text.muted' })}
          icon={publication.data.passwordUnlocked ? LockOpenIcon : LockIcon}
          size={14}
        />
      {/if}

      <h2
        class={css({
          flex: '1',
          minWidth: '0',
          fontSize: '17px',
          fontWeight: 'semibold',
          lineHeight: '[1.4]',
          letterSpacing: '-0.01em',
          lineClamp: '2',
        })}
      >
        <span class={cx(css({ transition: 'colors' }), 'publication-title')}>{publication.data.title}</span>
      </h2>
    </div>

    {#if publication.data.subtitle}
      <h3
        class={css({ marginTop: '2px', fontSize: '14px', fontWeight: 'medium', lineHeight: '[1.5]', color: 'text.muted', lineClamp: '1' })}
      >
        {publication.data.subtitle}
      </h3>
    {/if}

    {#if publication.data.excerpt}
      <p
        class={css({
          marginTop: '6px',
          fontFamily: 'prose',
          fontSize: '14px',
          lineHeight: '[1.6]',
          letterSpacing: '-0.005em',
          color: 'text.muted',
          lineClamp: '2',
        })}
      >
        {publication.data.excerpt}
      </p>
    {/if}

    {#if hasMeta}
      <div
        class={flex({
          alignItems: 'center',
          flexWrap: 'wrap',
          gap: '8px',
          marginTop: '10px',
          fontSize: '12px',
          color: 'text.hint',
          fontVariantNumeric: 'tabular-nums',
        })}
      >
        {#if date}
          <span>{date}</span>
        {/if}
        {#if collectionName}
          {#if date}<span class={css({ size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })}></span>{/if}
          <span>{collectionName}</span>
        {/if}
        {#if publication.data.tags.length > 0}
          {#if date || collectionName}<span
              class={css({ size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })}
            ></span>{/if}
          <span class={flex({ gap: '6px' })}>
            {#each publication.data.tags as tag (tag)}
              <span>#{tag}</span>
            {/each}
          </span>
        {/if}
      </div>
    {/if}
  </div>

  {#if publication.data.thumbnail}
    <div
      class={cx(
        css({
          flexShrink: '0',
          width: { base: '72px', md: '88px' },
          aspectRatio: '1/1',
          borderRadius: '6px',
          backgroundColor: 'surface.canvas',
          overflow: 'hidden',
          isolation: 'isolate',
        }),
        'publication-thumbnail',
      )}
    >
      <Img
        style={css.raw({
          width: 'full',
          height: 'full',
          objectFit: 'cover',
          transition: '[transform 240ms cubic-bezier(0.32, 0.72, 0, 1)]',
        })}
        alt={publication.data.title}
        image$key={publication.data.thumbnail}
        size={256}
      />
    </div>
  {/if}
</a>
