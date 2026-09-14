<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Img } from '$lib/components';
  import { splitHighlight } from '$lib/discovery/highlight-terms';
  import { hydrateQuery } from '$lib/graphql';
  import TagChip from '../@[slug]/TagChip.svelte';
  import { discoveryCardList } from './discovery-styles';
  import DiscoveryCard from './DiscoveryCard.svelte';
  import { discoveryTagPath } from './paths';
  import type { HydratableQuery } from '$lib/graphql';
  import type { UsersiteApexSearchPage_Query } from '$mearie';

  type Props = {
    q: string;
    searchQuery: HydratableQuery<UsersiteApexSearchPage_Query>;
  };

  let { q, searchQuery }: Props = $props();

  const query = $derived(hydrateQuery(() => searchQuery));
  const result = $derived(query.data.discovery.search);
  const empty = $derived(!result || (result.publications.length === 0 && result.spaces.length === 0 && result.tags.length === 0));

  const message = css.raw({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' });
  const heading = css.raw({
    marginBottom: { base: '10px', lg: '12px' },
    fontSize: { base: '15px', lg: '13px' },
    fontWeight: { base: 'bold', lg: 'semibold' },
    letterSpacing: { base: '-0.02em', lg: '0' },
  });
  const none = css.raw({ fontSize: '13px', color: 'text.hint' });
  const hit = css.raw({ fontWeight: 'semibold', color: 'text.default', backgroundColor: 'accent.subtle', borderRadius: '2px' });
</script>

{#snippet highlighted(text: string)}
  {#each splitHighlight(text, q) as part, index (index)}
    {#if part.hit}<mark class={css(hit)}>{part.text}</mark>{:else}{part.text}{/if}
  {/each}
{/snippet}

{#if result && !empty}
  <div
    class={css({
      marginBottom: { base: '24px', lg: '20px' },
      fontSize: '13px',
      color: 'text.hint',
      fontVariantNumeric: 'tabular-nums',
      lg: { gridColumn: '1', gridRow: '2' },
    })}
  >
    글 {result.publications.length}편 · 스페이스 {result.spaces.length}개 · 태그 {result.tags.length}개
  </div>
{/if}

<section class={css({ minWidth: '0', order: { base: '2', lg: '0' }, lg: { gridColumn: '1', gridRow: '3' } })}>
  {#if !result}
    <p class={css(message)}>검색 결과를 불러오지 못했어요.</p>
  {:else if empty}
    <p class={css(message)}>검색 결과가 없어요.</p>
  {:else}
    {#if result.publications.length > 0}
      <div class={css(discoveryCardList)}>
        {#each result.publications as hit (hit.publication.id)}
          <DiscoveryCard excerptHtml={hit.excerpt} publicationView$key={hit.publication} titleHtml={hit.title} />
        {/each}
      </div>
    {:else}
      <p class={css(message)}>일치하는 글이 없어요.</p>
    {/if}
  {/if}
</section>

{#if result && !empty}
  <aside
    class={flex({
      flexDirection: 'column',
      gap: { base: '24px', lg: '36px' },
      minWidth: '0',
      order: { base: '1', lg: '0' },
      lg: {
        position: 'sticky',
        top: '[calc(var(--usersite-sticky-header-bottom, 52px) + 28px)]',
        gridColumn: '2',
        gridRow: '1 / span 3',
      },
      lgDown: { marginBottom: '4px', paddingBottom: '24px', borderBottomWidth: '1px', borderColor: 'border.hairline' },
    })}
  >
    <div>
      <h2 class={css(heading)}>스페이스</h2>
      {#if result.spaces.length > 0}
        <div class={flex({ display: { base: 'none', lg: 'flex' }, flexDirection: 'column', marginY: '-8px' })}>
          {#each result.spaces as space (space.id)}
            <a
              class={flex({
                alignItems: 'center',
                gap: '12px',
                minWidth: '0',
                paddingY: '8px',
                _hover: { '& [data-row-name]': { color: 'text.muted' } },
              })}
              href={space.url}
            >
              <Img
                style={css.raw({
                  flexShrink: '0',
                  size: '36px',
                  borderRadius: '9px',
                  objectFit: 'cover',
                  boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
                })}
                alt={`${space.name} 로고`}
                image$key={space.logo}
                size={96}
              />
              <div class={css({ flex: '1', minWidth: '0' })}>
                <div
                  class={css({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.4]', transition: 'colors', truncate: true })}
                  data-row-name
                >
                  {@render highlighted(space.name)}
                </div>
                {#if space.description}
                  <div class={css({ fontSize: '12px', lineHeight: '[1.45]', color: 'text.muted', truncate: true })}>
                    {@render highlighted(space.description)}
                  </div>
                {/if}
              </div>
            </a>
          {/each}
        </div>
        <div class={flex({ display: { base: 'flex', lg: 'none' }, flexDirection: 'column', gap: '8px' })}>
          {#each result.spaces as space (space.id)}
            <a
              class={flex({
                alignItems: 'center',
                gap: '14px',
                minWidth: '0',
                paddingY: '12px',
                paddingX: '14px',
                borderRadius: '12px',
                backgroundColor: 'surface.inset',
                transition: 'colors',
                _hover: { backgroundColor: 'surface.hover' },
              })}
              href={space.url}
            >
              <Img
                style={css.raw({
                  flexShrink: '0',
                  size: '44px',
                  borderRadius: '11px',
                  objectFit: 'cover',
                  boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
                })}
                alt={`${space.name} 로고`}
                image$key={space.logo}
                size={96}
              />
              <div class={css({ minWidth: '0' })}>
                <div class={css({ fontSize: '15px', fontWeight: 'semibold', letterSpacing: '-0.01em', truncate: true })}>
                  {@render highlighted(space.name)}
                </div>
                {#if space.description}
                  <div class={css({ marginTop: '2px', fontSize: '13px', color: 'text.muted', lineClamp: '1' })}>
                    {@render highlighted(space.description)}
                  </div>
                {/if}
                <div class={css({ marginTop: '4px', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>
                  글 {space.publicationCount}개
                </div>
              </div>
            </a>
          {/each}
        </div>
      {:else}
        <p class={css(none)}>일치하는 스페이스가 없어요.</p>
      {/if}
    </div>

    <div>
      <h2 class={css(heading)}>태그</h2>
      {#if result.tags.length > 0}
        <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
          {#each result.tags as tag (tag.name)}
            <TagChip name={tag.name} count={tag.count} current={false} href={discoveryTagPath(tag.name)} noscroll={false} />
          {/each}
        </div>
      {:else}
        <p class={css(none)}>일치하는 태그가 없어요.</p>
      {/if}
    </div>
  </aside>
{/if}
