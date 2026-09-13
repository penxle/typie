<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Img } from '$lib/components';
  import { hydrateQuery } from '$lib/graphql';
  import PublicationListItem from '../@[slug]/PublicationListItem.svelte';
  import TagChip from '../@[slug]/TagChip.svelte';
  import { discoveryTagPath } from './paths';
  import type { HydratableQuery } from '$lib/graphql';
  import type { UsersiteApexSearchPage_Query } from '$mearie';

  type Props = {
    searchQuery: HydratableQuery<UsersiteApexSearchPage_Query>;
  };

  let { searchQuery }: Props = $props();

  const query = $derived(hydrateQuery(() => searchQuery));
  const result = $derived(query.data.discovery.search);
  const empty = $derived(!result || (result.publications.length === 0 && result.spaces.length === 0 && result.tags.length === 0));

  const heading = css.raw({ marginBottom: '12px', fontSize: '13px', fontWeight: 'semibold', color: 'text.hint' });
</script>

{#if !result}
  <p class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>검색 결과를 불러오지 못했어요.</p>
{:else if empty}
  <p class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>검색 결과가 없어요.</p>
{:else}
  <div class={flex({ flexDirection: 'column', gap: '32px' })}>
    {#if result.spaces.length > 0}
      <section>
        <h2 class={css(heading)}>스페이스</h2>
        <div class={flex({ flexDirection: 'column' })}>
          {#each result.spaces as space (space.id)}
            <a
              class={flex({
                alignItems: 'center',
                gap: '12px',
                paddingY: '10px',
                borderTopWidth: '1px',
                borderColor: 'border.hairline',
                _first: { borderTopWidth: '0' },
                _hover: { '& .space-name': { color: 'text.muted' } },
              })}
              href={space.url}
            >
              <Img
                style={css.raw({
                  flexShrink: '0',
                  size: '32px',
                  borderRadius: '8px',
                  objectFit: 'cover',
                  boxShadow: '[inset 0 0 0 1px rgba(0, 0, 0, 0.06)]',
                })}
                alt={`${space.name} 로고`}
                image$key={space.logo}
                size={64}
              />
              <div class={css({ minWidth: '0' })}>
                <div
                  class={css({
                    fontSize: '14px',
                    fontWeight: 'semibold',
                    letterSpacing: '-0.01em',
                    color: 'text.default',
                    transition: 'colors',
                    truncate: true,
                  })}
                >
                  <span class="space-name">{space.name}</span>
                </div>
                {#if space.description}
                  <div class={css({ marginTop: '2px', fontSize: '13px', color: 'text.muted', lineClamp: '1' })}>{space.description}</div>
                {/if}
              </div>
            </a>
          {/each}
        </div>
      </section>
    {/if}

    {#if result.tags.length > 0}
      <section>
        <h2 class={css(heading)}>태그</h2>
        <div class={flex({ flexWrap: 'wrap', gap: '8px' })}>
          {#each result.tags as tag (tag.name)}
            <TagChip name={tag.name} count={tag.count} current={false} href={discoveryTagPath(tag.name)} size="lg" />
          {/each}
        </div>
      </section>
    {/if}

    {#if result.publications.length > 0}
      <section>
        <h2 class={css(heading)}>글</h2>
        <div class={flex({ flexDirection: 'column' })}>
          {#each result.publications as hit, index (hit.publication.id)}
            <PublicationListItem
              dateDisplay="PUBLISHED_AT"
              excerptHtml={hit.excerpt}
              first={index === 0}
              publicationView$key={hit.publication}
              showSpace
              titleHtml={hit.title}
            />
          {/each}
        </div>
      </section>
    {/if}
  </div>
{/if}
