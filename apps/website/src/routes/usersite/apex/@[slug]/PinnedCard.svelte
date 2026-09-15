<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import { comma } from '@typie/ui/utils';
  import dayjs from 'dayjs';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import PinIcon from '~icons/lucide/pin';
  import SmileIcon from '~icons/lucide/smile';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { pickSpaceDate } from './space-date';
  import type { UsersiteSpace_PinnedCard_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpace_PinnedCard_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpace_PinnedCard_publicationView on PublicationView {
        id
        title
        excerpt
        publishedAt
        updatedAt
        reactionCount
        hasPassword
        passwordUnlocked
        url

        folder {
          id
          name
        }

        thumbnail {
          id
          ...Img_image
        }

        site {
          id
          dateDisplay
        }
      }
    `),
    () => publicationView$key,
  );

  const date = $derived(pickSpaceDate(publication.data.site.dateDisplay, publication.data));
  const timestamp = $derived(date === null ? null : dayjs(date).valueOf());
</script>

<a
  class={flex({
    flexDirection: 'column',
    minWidth: '0',
    padding: '[16px 18px]',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '12px',
    transition: 'common',
    _hover: { borderColor: 'border.default' },
  })}
  href={publication.data.url}
>
  <div class={flex({ alignItems: 'center', gap: '6px', minWidth: '0', fontSize: '12px', color: 'text.hint', whiteSpace: 'nowrap' })}>
    <span class={flex({ alignItems: 'center', gap: '3px', flexShrink: '0', fontWeight: 'semibold', color: 'text.muted' })}>
      <Icon icon={PinIcon} size={12} />
      고정
    </span>
    {#if publication.data.folder}
      <i class={css({ flexShrink: '0', size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })} aria-hidden="true"></i>
      <span class={css({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis' })}>{publication.data.folder.name}</span>
    {/if}
  </div>

  <div class={flex({ alignItems: 'flex-start', gap: '12px', marginTop: '8px' })}>
    <div class={css({ flex: '1', minWidth: '0' })}>
      <h3
        class={flex({
          alignItems: 'flex-start',
          gap: '5px',
          fontSize: '16px',
          fontWeight: 'bold',
          lineHeight: '[1.4]',
          letterSpacing: '-0.015em',
        })}
      >
        {#if publication.data.hasPassword}
          <Icon
            style={css.raw({ flexShrink: '0', marginTop: '4px', color: 'text.muted' })}
            icon={publication.data.passwordUnlocked ? LockOpenIcon : LockIcon}
            size={14}
          />
        {/if}
        <span class={css({ minWidth: '0', lineClamp: '2' })}>{publication.data.title}</span>
      </h3>

      {#if publication.data.excerpt}
        <p
          class={css({
            marginTop: '6px',
            fontFamily: 'prose',
            fontSize: '14px',
            lineHeight: '[1.6]',
            letterSpacing: '-0.005em',
            color: 'text.muted',
            lineClamp: '3',
          })}
        >
          {publication.data.excerpt}
        </p>
      {/if}
    </div>
    {#if publication.data.thumbnail}
      <div
        class={css({
          flexShrink: '0',
          width: '72px',
          aspectRatio: '[16 / 9]',
          borderRadius: '4px',
          backgroundColor: 'surface.canvas',
          overflow: 'hidden',
          isolation: 'isolate',
        })}
      >
        <Img
          style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
          alt={publication.data.title}
          image$key={publication.data.thumbnail}
          size={256}
        />
      </div>
    {/if}
  </div>

  {#if timestamp !== null || publication.data.reactionCount > 0}
    <div
      class={flex({
        alignItems: 'center',
        gap: '6px',
        marginTop: 'auto',
        paddingTop: '12px',
        fontSize: '12px',
        color: 'text.hint',
        fontVariantNumeric: 'tabular-nums',
      })}
    >
      {#if timestamp !== null}
        <TimeAgo {timestamp} />
      {/if}
      {#if publication.data.reactionCount > 0}
        <span class={flex({ alignItems: 'center', gap: '3px', flexShrink: '0', marginLeft: 'auto' })}>
          <Icon icon={SmileIcon} size={14} />
          {comma(publication.data.reactionCount)}
        </span>
      {/if}
    </div>
  {/if}
</a>
