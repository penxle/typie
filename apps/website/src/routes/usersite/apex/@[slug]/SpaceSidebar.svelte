<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { twoWaySticky } from '../(site)/two-way-sticky';
  import { currentSpaceSlug } from './current-space-slug';
  import { seriesPath, tagPath } from './paths';
  import SpaceHeader from './SpaceHeader.svelte';
  import TagChip from './TagChip.svelte';
  import type { UsersiteSpace_SpaceHeader_spaceView$key, UsersiteSpace_SpaceSidebar_spaceView$key } from '$mearie';

  type Props = {
    spaceView$key: UsersiteSpace_SpaceSidebar_spaceView$key & UsersiteSpace_SpaceHeader_spaceView$key;
    activeCollectionId: string | null;
    activeTag: string | null;
    headerBottom: number;
    viewportHeight: number;
  };

  let { spaceView$key, activeCollectionId, activeTag, headerBottom, viewportHeight }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteSpace_SpaceSidebar_spaceView on SpaceView {
        id

        collections {
          id
          permalink
          name

          cover {
            id
            ...Img_image
          }

          publications {
            id
          }
        }

        tags {
          name
          count
        }
      }
    `),
    () => spaceView$key,
  );

  const slug = $derived(currentSpaceSlug());

  const heading = css.raw({ marginBottom: '8px', fontSize: '14px', fontWeight: 'bold', letterSpacing: '-0.01em' });
  const rowSub = css.raw({ fontSize: '12px', lineHeight: '[1.45]', color: 'text.hint', truncate: true });
  const desktopOnly = css.raw({ lgDown: { display: 'none' } });
</script>

<aside
  class={flex({
    flexDirection: 'column',
    gap: '36px',
    minWidth: '0',
    lg: { position: 'sticky', top: '[calc(var(--usersite-sticky-header-bottom, 52px) + var(--usersite-space-tabs-height, 45px) + 28px)]' },
  })}
  use:twoWaySticky={{ headerBottom, viewportHeight }}
>
  <div class={css(desktopOnly)}>
    <SpaceHeader {spaceView$key} variant="sidebar" />
  </div>

  {#if space.data.collections.length > 0}
    <div class={css(desktopOnly)}>
      <h3 class={css(heading)}>시리즈</h3>
      {#each space.data.collections as collection (collection.id)}
        <a
          class={flex({
            alignItems: 'center',
            gap: '12px',
            minWidth: '0',
            paddingY: '8px',
            _hover: { '& [data-row-name]': { color: 'text.muted' } },
            _currentPage: { marginX: '-10px', paddingX: '10px', borderRadius: '10px', backgroundColor: 'surface.inset' },
          })}
          aria-current={collection.id === activeCollectionId ? 'page' : undefined}
          href={seriesPath(slug, collection.permalink)}
        >
          <div
            style:background-color={collection.cover ? undefined : titlePageColors(collection.name).base}
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
            {#if collection.cover}
              <Img
                style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                alt={collection.name}
                image$key={collection.cover}
                size={96}
              />
            {/if}
          </div>
          <div class={css({ flex: '1', minWidth: '0' })}>
            <div
              class={css({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true })}
              data-row-name
            >
              {collection.name}
            </div>
            <div class={css(rowSub)}>글 {collection.publications.length}개</div>
          </div>
        </a>
      {/each}
    </div>
  {/if}

  {#if space.data.tags.length > 0}
    <div class={css(desktopOnly)}>
      <h3 class={css(heading)}>태그</h3>
      <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
        {#each space.data.tags as tag (tag.name)}
          <TagChip name={tag.name} count={tag.count} current={tag.name === activeTag} href={tagPath(slug, tag.name)} noscroll={false} />
        {/each}
      </div>
    </div>
  {/if}
</aside>
