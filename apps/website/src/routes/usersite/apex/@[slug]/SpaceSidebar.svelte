<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { graphql } from '$mearie';
  import { twoWaySticky } from '../(site)/two-way-sticky';
  import { currentSpaceSlug } from './current-space-slug';
  import { tagPath } from './paths';
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
