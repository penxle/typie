<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getUsersiteChrome } from '../../chrome.svelte';
  import { stuck } from '../@[slug]/stuck';
  import TagChip from '../@[slug]/TagChip.svelte';
  import { discoveryTagPath } from './paths';

  type Props = {
    tags: readonly { name: string; count: number }[];
    active: string | null;
  };

  let { tags, active }: Props = $props();

  const chrome = getUsersiteChrome();
</script>

{#if tags.length > 0}
  <div
    class={css({
      position: 'sticky',
      top: '[var(--usersite-sticky-header-bottom, 0px)]',
      zIndex: '10',
      marginX: { base: '-20px', md: '-40px' },
      paddingX: { base: '20px', md: '40px' },
      paddingY: '12px',
      borderBottomWidth: '1px',
      borderColor: 'border.hairline',
      backgroundColor: 'surface.default',
      transition: '[border-color 150ms ease-out]',
      '&[data-stuck]': { borderColor: 'border.default' },
    })}
    aria-label="태그"
    use:stuck={{ onchange: (node, value) => chrome.setStuck(node, value) }}
  >
    <div
      class={css({ display: 'flex', gap: '8px', overflowX: 'auto', scrollbarWidth: 'none', '&::-webkit-scrollbar': { display: 'none' } })}
    >
      {#each tags as tag (tag.name)}
        <span class={css({ flexShrink: '0' })}>
          <TagChip name={tag.name} count={tag.count} current={tag.name === active} href={discoveryTagPath(tag.name)} size="lg" />
        </span>
      {/each}
    </div>
  </div>
{/if}
