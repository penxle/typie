<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { page } from '$app/state';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { twoWaySticky } from '../(site)/two-way-sticky';
  import { currentSpaceSlug } from './current-space-slug';
  import { folderPath, tagPath } from './paths';
  import SpaceHeader from './SpaceHeader.svelte';
  import TagChip from './TagChip.svelte';
  import type { UsersiteSpace_SpaceHeader_siteView$key, UsersiteSpace_SpaceSidebar_siteView$key } from '$mearie';

  type Props = {
    siteView$key: UsersiteSpace_SpaceSidebar_siteView$key & UsersiteSpace_SpaceHeader_siteView$key;
    activeTag: string | null;
    hiddenOnMobile?: boolean;
    headerBottom: number;
    viewportHeight: number;
  };

  let { siteView$key, activeTag, hiddenOnMobile = false, headerBottom, viewportHeight }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment UsersiteSpace_SpaceSidebar_siteView on SiteView {
        id

        pinnedFolders {
          id
          number
          name
          publicationCount

          thumbnail {
            id
            ...Img_image
          }
        }

        tags {
          name
          count
        }
      }
    `),
    () => siteView$key,
  );

  const slug = $derived(currentSpaceSlug());

  const heading = css.raw({ marginBottom: '8px', fontSize: '14px', fontWeight: 'bold', letterSpacing: '-0.01em' });
</script>

<aside
  class={css(
    flex.raw({
      flexDirection: 'column',
      gap: '36px',
      minWidth: '0',
      lg: { position: 'sticky', top: '[calc(var(--usersite-sticky-header-bottom, 52px) + 28px)]' },
      lgDown: { marginTop: '16px', paddingTop: '40px', borderTopWidth: '1px', borderColor: 'border.hairline' },
    }),
    hiddenOnMobile && { lgDown: { display: 'none' } },
  )}
  use:twoWaySticky={{ headerBottom, viewportHeight }}
>
  <div class={css({ lgDown: { display: 'none' } })}>
    <SpaceHeader {siteView$key} variant="sidebar" />
  </div>

  {#if site.data.pinnedFolders.length > 0}
    <div>
      <h3 class={css(heading)}>대표 시리즈</h3>
      {#each site.data.pinnedFolders as folder (folder.id)}
        <a
          class={flex({
            alignItems: 'center',
            gap: '12px',
            minWidth: '0',
            paddingY: '8px',
            _hover: { '& [data-row-name]': { color: 'text.muted' } },
            _currentPage: { marginX: '-10px', paddingX: '10px', borderRadius: '10px', backgroundColor: 'surface.inset' },
          })}
          aria-current={page.url.pathname === folderPath(slug, folder.number) ? 'page' : undefined}
          href={folderPath(slug, folder.number)}
        >
          <div
            style:background-color={folder.thumbnail ? undefined : titlePageColors(folder.id).background}
            class={css({
              flexShrink: '0',
              width: '36px',
              height: '54px',
              borderRadius: '4px',
              backgroundColor: 'surface.inset',
              boxShadow: '[inset 0 0 0 1px token(colors.border.hairline)]',
              overflow: 'hidden',
            })}
          >
            {#if folder.thumbnail}
              <Img
                style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                alt={folder.name}
                image$key={folder.thumbnail}
                size={96}
              />
            {/if}
          </div>
          <div class={css({ flex: '1', minWidth: '0' })}>
            <div
              class={css({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true })}
              data-row-name
            >
              {folder.name}
            </div>
            <div class={css({ marginTop: '2px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
              글 {folder.publicationCount}개
            </div>
          </div>
        </a>
      {/each}
    </div>
  {/if}

  {#if site.data.tags.length > 0}
    <div>
      <h3 class={css(heading)}>태그</h3>
      <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
        {#each site.data.tags as tag (tag.name)}
          <TagChip name={tag.name} count={tag.count} current={tag.name === activeTag} href={tagPath(slug, tag.name)} noscroll={false} />
        {/each}
      </div>
    </div>
  {/if}
</aside>
