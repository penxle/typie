<script lang="ts">
  import { css } from '@typie/styled-system/css';
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
  <header class={css({ marginBottom: '32px' })}>
    {#if folder.thumbnail}
      <div
        class={css({
          marginBottom: '20px',
          aspectRatio: '[16 / 9]',
          '@media (max-width: 639px)': { marginBottom: '16px' },
          borderRadius: '12px',
          backgroundColor: 'surface.canvas',
          boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.04)]',
          overflow: 'hidden',
          isolation: 'isolate',
        })}
      >
        <Img
          style={css.raw({ width: 'full', height: 'full', objectFit: 'cover' })}
          alt={folder.name}
          image$key={folder.thumbnail}
          size={1024}
        />
      </div>
    {/if}

    <ReadingKicker items={crumbs} />

    <h1
      class={css({
        marginTop: '10px',
        fontSize: '24px',
        fontWeight: 'bold',
        letterSpacing: '-0.02em',
        lineHeight: '[1.3]',
        lineClamp: '2',
      })}
    >
      {folder.name}
    </h1>
    <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
      {countLabel}
    </p>
  </header>

  <SiteEntries entries={folder.children} />
{/key}
