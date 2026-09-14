<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { groupTagsByInitial } from '$lib/discovery/tag-initials';
  import { hydrateQuery } from '$lib/graphql';
  import { getUsersiteChrome } from '../../../../chrome.svelte';
  import { stuck } from '../../../@[slug]/stuck';
  import TagChip from '../../../@[slug]/TagChip.svelte';
  import DiscoveryPageHead from '../../DiscoveryPageHead.svelte';
  import { discoveryTagPath } from '../../paths';
  import type { TagInitial } from '$lib/discovery/tag-initials';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.tagsQuery));
  const tags = $derived(query.data.discovery.allTags);
  const groups = $derived(groupTagsByInitial(tags));

  const chrome = getUsersiteChrome();

  const GAP = 20;

  let bar = $state<HTMLElement>();

  const sectionId = (initial: TagInitial) => `tags-${encodeURIComponent(initial)}`;

  const jump = (e: MouseEvent, initial: TagInitial) => {
    const section = document.querySelector<HTMLElement>(`#${CSS.escape(sectionId(initial))}`);
    if (!section) return;
    e.preventDefault();
    const top = window.scrollY + section.getBoundingClientRect().top;
    const offset = chrome.headerHeight + (bar?.getBoundingClientRect().height ?? 0) + GAP;
    window.scrollTo({ top: top - offset, behavior: prefersReducedMotion.current ? 'auto' : 'smooth' });
  };
</script>

<Helmet title="태그" />

<DiscoveryPageHead sub={`태그 ${tags.length}개`} title="태그" />

{#if groups.length > 0}
  <nav
    bind:this={bar}
    class={flex({
      position: 'sticky',
      top: '[var(--usersite-sticky-header-bottom, 0px)]',
      zIndex: '5',
      gap: '2px',
      marginTop: { base: '12px', md: '16px' },
      marginX: { base: '-20px', md: '0' },
      paddingY: '8px',
      paddingX: { base: '20px', md: '0' },
      borderBottomWidth: '1px',
      borderColor: 'border.hairline',
      backgroundColor: 'surface.default',
      flexWrap: { base: 'nowrap', md: 'wrap' },
      overflowX: { base: 'auto', md: 'visible' },
      scrollbarWidth: 'none',
      '&::-webkit-scrollbar': { display: 'none' },
    })}
    aria-label="초성"
    use:stuck={{ onchange: (node, value) => chrome.setStuck(node, value) }}
  >
    {#each groups as group (group.initial)}
      <a
        class={flex({
          alignItems: 'center',
          justifyContent: 'center',
          flexShrink: '0',
          minWidth: '32px',
          height: '30px',
          paddingX: '6px',
          borderRadius: '6px',
          fontSize: '14px',
          fontWeight: 'semibold',
          color: 'text.muted',
          transition: 'colors',
          _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
        })}
        href={`#${sectionId(group.initial)}`}
        onclick={(e) => jump(e, group.initial)}
      >
        {group.initial}
      </a>
    {/each}
  </nav>

  <div class={flex({ flexDirection: 'column' })}>
    {#each groups as group (group.initial)}
      <section
        id={sectionId(group.initial)}
        class={css({
          display: 'grid',
          gridTemplateColumns: { base: '36px minmax(0, 1fr)', md: '48px minmax(0, 1fr)' },
          gap: { base: '10px', md: '16px' },
          paddingY: '18px',
          borderBottomWidth: '1px',
          borderColor: 'border.hairline',
        })}
      >
        <h2 class={css({ fontSize: { base: '16px', md: '20px' }, fontWeight: 'bold', lineHeight: '[26px]', whiteSpace: 'nowrap' })}>
          {group.initial}
        </h2>
        <div class={flex({ flexWrap: 'wrap', gap: '6px' })}>
          {#each group.tags as tag (tag.name)}
            <TagChip name={tag.name} count={tag.count} current={false} href={discoveryTagPath(tag.name)} noscroll={false} />
          {/each}
        </div>
      </section>
    {/each}
  </div>
{:else}
  <p class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>아직 태그가 없어요</p>
{/if}
