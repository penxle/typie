<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Img } from '$lib/components';
  import { graphql } from '$mearie';
  import { seriesPath, tagPath } from './paths';
  import SpaceHeader from './SpaceHeader.svelte';
  import TagChip from './TagChip.svelte';
  import type { UsersiteWildcard_SpaceHeader_spaceView$key, UsersiteWildcard_SpaceRail_spaceView$key } from '$mearie';

  type Props = {
    spaceView$key: UsersiteWildcard_SpaceRail_spaceView$key & UsersiteWildcard_SpaceHeader_spaceView$key;
    activeCollectionId: string | null;
    activeTag: string | null;
  };

  let { spaceView$key, activeCollectionId, activeTag }: Props = $props();

  const space = createFragment(
    graphql(`
      fragment UsersiteWildcard_SpaceRail_spaceView on SpaceView {
        id

        collections {
          id
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
</script>

<div class={flex({ flexDirection: 'column' })}>
  <div class={css({ paddingBottom: '20px' })}>
    <SpaceHeader {spaceView$key} variant="rail" />
  </div>

  {#if space.data.collections.length > 0}
    <div class={css({ paddingY: '16px', borderTopWidth: '1px', borderColor: 'border.hairline' })}>
      <div class={css({ marginBottom: '8px', fontSize: '13px', fontWeight: 'semibold' })}>시리즈</div>
      <div class={flex({ flexDirection: 'column', gap: '2px', marginX: '-8px' })}>
        {#each space.data.collections as collection (collection.id)}
          <a
            class={flex({
              alignItems: 'center',
              gap: '10px',
              height: '36px',
              paddingX: '8px',
              borderRadius: '8px',
              fontSize: '13px',
              textAlign: 'left',
              transition: 'common',
              _hover: { backgroundColor: 'surface.hover' },
              _currentPage: { backgroundColor: 'surface.inset' },
            })}
            aria-current={collection.id === activeCollectionId ? 'page' : undefined}
            data-sveltekit-noscroll
            href={collection.id === activeCollectionId ? '/' : seriesPath(collection.id)}
          >
            <div class={css({ flexShrink: '0', size: '24px', borderRadius: '6px', backgroundColor: 'surface.canvas', overflow: 'hidden' })}>
              {#if collection.cover}
                <Img
                  style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
                  alt={collection.name}
                  image$key={collection.cover}
                  size={48}
                />
              {/if}
            </div>
            <span class={css({ flex: '1', minWidth: '0', fontWeight: 'medium', lineClamp: '1' })}>{collection.name}</span>
            <span class={css({ fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
              {collection.publications.length}
            </span>
          </a>
        {/each}
      </div>
    </div>
  {/if}

  {#if space.data.tags.length > 0}
    <div class={css({ paddingY: '16px', borderTopWidth: '1px', borderColor: 'border.hairline' })}>
      <div class={css({ marginBottom: '8px', fontSize: '13px', fontWeight: 'semibold' })}>태그</div>
      <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
        {#each space.data.tags as tag (tag.name)}
          <TagChip
            name={tag.name}
            count={tag.count}
            current={tag.name === activeTag}
            href={tag.name === activeTag ? '/' : tagPath(tag.name)}
          />
        {/each}
      </div>
    </div>
  {/if}
</div>
