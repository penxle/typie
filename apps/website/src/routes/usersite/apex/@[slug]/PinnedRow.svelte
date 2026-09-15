<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, TimeAgo } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import LockIcon from '~icons/lucide/lock';
  import LockOpenIcon from '~icons/lucide/lock-open';
  import PinIcon from '~icons/lucide/pin';
  import { graphql } from '$mearie';
  import { pickSpaceDate } from './space-date';
  import type { UsersiteSpace_PinnedRow_publicationView$key } from '$mearie';

  type Props = {
    publicationView$key: UsersiteSpace_PinnedRow_publicationView$key;
  };

  let { publicationView$key }: Props = $props();

  const publication = createFragment(
    graphql(`
      fragment UsersiteSpace_PinnedRow_publicationView on PublicationView {
        id
        title
        publishedAt
        updatedAt
        hasPassword
        passwordUnlocked
        url

        folder {
          id
          name
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
  const hasMeta = $derived(!!publication.data.folder || timestamp !== null);

  const narrowHidden = css.raw({ '@media (max-width: 639px)': { display: 'none' } });
</script>

<a
  class={flex({
    alignItems: 'center',
    gap: '12px',
    minWidth: '0',
    paddingY: '11px',
    borderTopWidth: '1px',
    borderColor: 'border.hairline',
    _first: { paddingTop: '2px', borderTopWidth: '0' },
    _hover: { '& [data-row-title]': { color: 'text.muted' } },
  })}
  href={publication.data.url}
>
  <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={PinIcon} size={14} />

  <span
    class={flex({
      flex: '1',
      alignItems: 'center',
      gap: '6px',
      minWidth: '0',
      fontSize: '15px',
      fontWeight: 'semibold',
      lineHeight: '[1.4]',
      letterSpacing: '-0.01em',
    })}
  >
    {#if publication.data.hasPassword}
      <Icon
        style={css.raw({ flexShrink: '0', color: 'text.muted' })}
        icon={publication.data.passwordUnlocked ? LockOpenIcon : LockIcon}
        size={14}
      />
    {/if}
    <span class={css({ minWidth: '0', transition: 'colors', truncate: true })} data-row-title>{publication.data.title}</span>
  </span>

  {#if hasMeta}
    <span
      class={flex({
        alignItems: 'center',
        flexShrink: '0',
        gap: '6px',
        fontSize: '12px',
        color: 'text.hint',
        fontVariantNumeric: 'tabular-nums',
        whiteSpace: 'nowrap',
      })}
    >
      {#if publication.data.folder}
        <span class={css(narrowHidden, { fontWeight: 'medium', color: 'text.muted' })}>{publication.data.folder.name}</span>
        {#if timestamp !== null}
          <i class={css(narrowHidden, { size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })} aria-hidden="true"></i>
        {/if}
      {/if}
      {#if timestamp !== null}
        <TimeAgo {timestamp} />
      {/if}
    </span>
  {/if}
</a>
