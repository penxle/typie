<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
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
  class={css({
    display: 'block',
    minWidth: '0',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '8px',
    overflow: 'hidden',
    isolation: 'isolate',
    _hover: {
      '& [data-folder-name]': { color: 'text.muted' },
      '& [data-folder-cover]': { transform: 'scale(1.03)' },
    },
  })}
  href={folderPath(slug, folder.data.number)}
>
  <div class={css({ aspectRatio: '[16 / 9]', backgroundColor: 'surface.canvas', color: 'text.hint', overflow: 'hidden' })}>
    <div
      class={flex({
        alignItems: 'center',
        justifyContent: 'center',
        width: 'full',
        height: 'full',
        transition: '[transform 240ms cubic-bezier(0.32, 0.72, 0, 1)]',
        _motionReduce: { transition: '[none]' },
      })}
      data-folder-cover
    >
      {#if folder.data.thumbnail}
        <Img
          style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
          alt={folder.data.name}
          image$key={folder.data.thumbnail}
          size={512}
        />
      {:else}
        <Icon icon={FolderIcon} size={24} />
      {/if}
    </div>
  </div>

  <div class={flex({ alignItems: 'center', gap: '8px', minWidth: '0', padding: '[10px 12px 11px]' })}>
    <div class={css({ flex: '1', minWidth: '0' })}>
      <p
        class={css({
          fontSize: '14px',
          fontWeight: 'semibold',
          lineHeight: '[1.35]',
          letterSpacing: '-0.01em',
          transition: 'colors',
          truncate: true,
        })}
        data-folder-name
      >
        {folder.data.name}
      </p>
      <p class={css({ marginTop: '1px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
        {folderCountLabel(folder.data.folderCount, folder.data.publicationCount)}
      </p>
    </div>

    <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={ChevronRightIcon} size={16} />
  </div>
</a>
