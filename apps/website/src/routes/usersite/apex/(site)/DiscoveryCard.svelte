<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import SmileIcon from '~icons/lucide/smile';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { seriesPath, tagPath } from '../@[slug]/paths';
  import { discoveryTagPath } from './paths';
  import type { UsersiteApex_DiscoveryCard_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteApex_DiscoveryCard_publicationView$key;
    enter?: boolean;
    context?: 'discovery' | 'space';
    showCollection?: boolean;
    titleHtml?: string | null;
    excerptHtml?: string | null;
  };

  let {
    publicationView$key,
    enter = false,
    context = 'discovery',
    showCollection = true,
    titleHtml = null,
    excerptHtml = null,
  }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteApex_DiscoveryCard_publicationView on PublicationView {
        id
        permalink
        title
        subtitle
        excerpt
        tags
        publishedAt
        reactionCount
        hasPassword
        passwordUnlocked
        url

        collection {
          id
          name
          permalink
        }

        thumbnail {
          id
          ...Img_image
        }

        space {
          id
          name
          slug
          url

          logo {
            id
            ...Img_image
          }
        }
      }
    `),
    () => publicationView$key,
  );

  const publishedAt = $derived(dayjs(publication.data.publishedAt).valueOf());
  const tagHref = (tag: string) => (context === 'space' ? tagPath(publication.data.space.slug, tag) : discoveryTagPath(tag));
  const hasExcerpt = $derived(!!(excerptHtml || publication.data.excerpt));

  const meta = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    minWidth: '0',
    fontSize: '12px',
    color: 'text.hint',
    fontVariantNumeric: 'tabular-nums',
    whiteSpace: 'nowrap',
  });

  const dot = css.raw({ flexShrink: '0', size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' });
  const highlight = css.raw({ '& em': { fontStyle: 'normal', color: 'accent.default' } });
</script>

<article
  class={css({
    minWidth: '0',
    paddingY: '24px',
    borderTopWidth: '1px',
    borderColor: 'border.hairline',
    _first: { paddingTop: '4px', borderTopWidth: '0' },
  })}
  in:fly={enter ? { y: 6, duration: 220, easing: cubicOut } : { duration: 0 }}
>
  <div class={css({ minWidth: '0' })}>
    <div class={css(meta, { height: '20px', marginBottom: '8px' })}>
      {#if context === 'space'}
        {#if showCollection && publication.data.collection}
          <a
            class={css({
              minWidth: '0',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              fontWeight: 'medium',
              color: 'text.muted',
              transition: 'colors',
              _hover: { color: 'text.default' },
            })}
            href={seriesPath(publication.data.space.slug, publication.data.collection.permalink)}
          >
            {publication.data.collection.name}
          </a>
          <i class={css(dot)} aria-hidden="true"></i>
        {/if}
      {:else}
        <a
          class={flex({
            alignItems: 'center',
            gap: '6px',
            minWidth: '0',
            fontWeight: 'medium',
            color: 'text.muted',
            _hover: { '& span': { color: 'text.default' } },
          })}
          href={publication.data.space.url}
        >
          <Img
            style={css.raw({
              flexShrink: '0',
              size: '20px',
              borderRadius: '5px',
              objectFit: 'cover',
              boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
            })}
            alt={`${publication.data.space.name} 로고`}
            image$key={publication.data.space.logo}
            size={48}
          />
          <span class={css({ overflow: 'hidden', textOverflow: 'ellipsis', transition: 'colors' })}>{publication.data.space.name}</span>
        </a>
        <i class={css(dot)} aria-hidden="true"></i>
      {/if}
      <TimeAgo timestamp={publishedAt} />
    </div>

    <a
      class={flex({
        alignItems: 'flex-start',
        gap: '14px',
        minWidth: '0',
        _hover: {
          '& [data-card-title]': { color: 'text.muted' },
          '& [data-card-cover]': { transform: 'scale(1.03)' },
        },
      })}
      href={publication.data.url}
    >
      <div class={css({ flex: '1', minWidth: '0', maxWidth: '640px' })}>
        <h3
          class={flex({
            alignItems: 'flex-start',
            gap: '6px',
            fontSize: '17px',
            fontWeight: 'semibold',
            lineHeight: '[1.35]',
            letterSpacing: '-0.015em',
            '@media (max-width: 639px)': { fontSize: '16px' },
          })}
        >
          {#if publication.data.hasPassword}
            <Icon
              style={css.raw({ flexShrink: '0', marginTop: '4px', color: 'text.muted' })}
              icon={publication.data.passwordUnlocked ? LockOpenIcon : LockIcon}
              size={14}
            />
          {/if}
          <span class={css(highlight, { minWidth: '0', transition: 'colors', lineClamp: '2' })} data-card-title>
            {#if titleHtml}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html titleHtml}
            {:else}
              {publication.data.title}
            {/if}
          </span>
        </h3>

        {#if publication.data.subtitle}
          <h3
            class={css({
              marginTop: '3px',
              fontSize: '14px',
              fontWeight: 'medium',
              lineHeight: '[1.5]',
              color: 'text.muted',
              lineClamp: '1',
            })}
          >
            {publication.data.subtitle}
          </h3>
        {/if}

        {#if hasExcerpt}
          <p
            class={css(highlight, {
              marginTop: '8px',
              fontFamily: 'prose',
              fontSize: '14px',
              lineHeight: '[1.6]',
              letterSpacing: '-0.005em',
              color: 'text.muted',
              lineClamp: '2',
            })}
          >
            {#if excerptHtml}
              <!-- eslint-disable-next-line svelte/no-at-html-tags -->
              {@html excerptHtml}
            {:else}
              {publication.data.excerpt}
            {/if}
          </p>
        {/if}
      </div>

      {#if publication.data.thumbnail}
        <div
          class={css({
            flexShrink: '0',
            width: '128px',
            marginTop: '3px',
            marginLeft: 'auto',
            aspectRatio: '[16 / 9]',
            borderRadius: '4px',
            backgroundColor: 'surface.canvas',
            overflow: 'hidden',
            isolation: 'isolate',
            '@media (max-width: 639px)': { width: '104px' },
          })}
        >
          <Img
            style={css.raw({
              width: 'full',
              height: 'full',
              objectFit: 'cover',
              transition: '[transform 240ms cubic-bezier(0.32, 0.72, 0, 1)]',
              _motionReduce: { transition: '[none]' },
            })}
            alt={publication.data.title}
            data-card-cover
            image$key={publication.data.thumbnail}
            size={256}
          />
        </div>
      {/if}
    </a>

    {#if publication.data.tags.length > 0 || publication.data.reactionCount > 0}
      <div class={css(meta, { alignItems: 'flex-start', marginTop: '12px', whiteSpace: 'normal' })}>
        {#if publication.data.tags.length > 0}
          <span class={flex({ flexWrap: 'wrap', columnGap: '6px', rowGap: '4px', minWidth: '0' })}>
            {#each publication.data.tags as tag (tag)}
              <a class={css({ flexShrink: '0', transition: 'colors', _hover: { color: 'text.default' } })} href={tagHref(tag)}>#{tag}</a>
            {/each}
          </span>
        {/if}
        {#if publication.data.reactionCount > 0}
          <span class={flex({ alignItems: 'center', gap: '3px', flexShrink: '0', marginLeft: 'auto' })}>
            <Icon icon={SmileIcon} size={14} />
            {comma(publication.data.reactionCount)}
          </span>
        {/if}
      </div>
    {/if}
  </div>
</article>
