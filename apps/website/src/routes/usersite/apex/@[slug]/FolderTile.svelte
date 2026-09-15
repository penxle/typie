<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { currentSpaceSlug } from './current-space-slug';
  import { folderCountLabel } from './folder-count';
  import { folderPath } from './paths';
  import type { UsersiteSpace_FolderTile_folderView$key } from '$mearie';

  type Props = {
    folderView$key: UsersiteSpace_FolderTile_folderView$key;
  };

  let { folderView$key }: Props = $props();

  const folder = createFragment(
    graphql(`
      fragment UsersiteSpace_FolderTile_folderView on SiteFolderView {
        id
        number
        name
        folderCount
        publicationCount
        description

        thumbnail {
          id
          ...Img_image
        }
      }
    `),
    () => folderView$key,
  );

  const slug = $derived(currentSpaceSlug());
</script>

<a
  class={flex({
    alignItems: 'center',
    gap: '16px',
    minWidth: '0',
    paddingY: '14px',
    borderTopWidth: '1px',
    borderColor: 'border.hairline',
    _first: { paddingTop: '4px', borderTopWidth: '0' },
    _hover: { '& [data-folder-name]': { color: 'text.muted' } },
  })}
  href={folderPath(slug, folder.data.number)}
>
  <div
    style:background-color={folder.data.thumbnail ? undefined : titlePageColors(folder.data.id).background}
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
    {#if folder.data.thumbnail}
      <Img
        style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
        alt={folder.data.name}
        image$key={folder.data.thumbnail}
        size={128}
      />
    {/if}
  </div>

  <div class={css({ flex: '1', minWidth: '0' })}>
    <p
      class={css({
        fontSize: '16px',
        fontWeight: 'semibold',
        lineHeight: '[1.4]',
        letterSpacing: '-0.01em',
        transition: 'colors',
        truncate: true,
      })}
      data-folder-name
    >
      {folder.data.name}
    </p>
    {#if folder.data.description}
      <p class={css({ marginTop: '4px', fontSize: '13px', lineHeight: '[1.5]', color: 'text.muted', lineClamp: '2' })}>
        {folder.data.description}
      </p>
    {/if}
    <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums', truncate: true })}>
      {folderCountLabel(folder.data.folderCount, folder.data.publicationCount)}
    </p>
  </div>

  <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={ChevronRightIcon} size={16} />
</a>
