<script lang="ts">
  import { titlePageColors } from '@typie/lib/title-page';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { Img } from '$lib/components';
  import { hydrateQuery } from '$lib/graphql';
  import ReadingKicker from '$lib/usersite/ReadingKicker.svelte';
  import { currentSpaceSlug } from '../../../current-space-slug';
  import { folderCountLabel } from '../../../folder-count';
  import { folderPath, spaceHomePath } from '../../../paths';
  import SiteEntries from '../../../SiteEntries.svelte';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.folderQuery));
  const site = $derived(query.data.siteView);
  const folder = $derived(site.folder);
  const slug = $derived(currentSpaceSlug());

  const countLabel = $derived(folderCountLabel(folder.folderCount, folder.publicationCount));
  const crumbs = $derived([
    { label: site.name, href: spaceHomePath(slug) },
    ...folder.ancestors.map((ancestor) => ({ label: ancestor.name, href: folderPath(slug, ancestor.number) })),
  ]);
</script>

<Helmet description={`${folder.name}의 ${countLabel}`} title={folder.name} trailing={site.name} />

{#key folder.id}
  <header
    class={flex({
      alignItems: 'center',
      gap: '24px',
      marginBottom: '40px',
      '@media (max-width: 639px)': { flexDirection: 'column', alignItems: 'flex-start', gap: '0' },
    })}
  >
    <div
      style:background-color={folder.thumbnail ? undefined : titlePageColors(folder.id).background}
      class={css({
        flexShrink: '0',
        width: '112px',
        height: '168px',
        borderRadius: '8px',
        backgroundColor: 'surface.inset',
        boxShadow: '[inset 0 0 0 1px token(colors.border.hairline)]',
        overflow: 'hidden',
        '@media (max-width: 639px)': { width: '80px', height: '120px', borderRadius: '6px' },
      })}
    >
      {#if folder.thumbnail}
        <Img
          style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
          alt={folder.name}
          image$key={folder.thumbnail}
          size={256}
        />
      {/if}
    </div>

    <div class={css({ flex: '1', minWidth: '0', '@media (max-width: 639px)': { flex: 'none', width: 'full', marginTop: '16px' } })}>
      <ReadingKicker items={crumbs} />

      <h1
        class={css({
          marginTop: '12px',
          fontSize: '28px',
          fontWeight: 'bold',
          letterSpacing: '-0.025em',
          lineHeight: '[1.25]',
          lineClamp: '2',
          '@media (max-width: 639px)': { marginTop: '8px', fontSize: '24px' },
        })}
      >
        {folder.name}
      </h1>
      {#if folder.description}
        <p
          class={css({
            marginTop: '10px',
            maxWidth: '560px',
            fontSize: '14px',
            lineHeight: '[1.6]',
            color: 'text.muted',
            '@media (max-width: 639px)': { marginTop: '8px', fontSize: '14px' },
          })}
        >
          {folder.description}
        </p>
      {/if}
      <p class={css({ marginTop: '10px', fontSize: '13px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
        {countLabel}
      </p>
    </div>
  </header>

  <SiteEntries entries={folder.children} nested />
{/key}
