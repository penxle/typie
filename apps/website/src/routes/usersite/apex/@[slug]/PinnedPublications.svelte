<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import DiscoverySectionHead from '../(site)/DiscoverySectionHead.svelte';
  import PinnedCard from './PinnedCard.svelte';
  import type { UsersiteSpace_PinnedCard_publicationView$key } from '$mearie';

  type Props = {
    publications: readonly ({ id: string } & UsersiteSpace_PinnedCard_publicationView$key)[];
  };

  let { publications }: Props = $props();
</script>

<section>
  <DiscoverySectionHead title="고정 글" />

  <div
    class={css({
      display: 'grid',
      gridTemplateColumns: 'repeat(3, minmax(0, 1fr))',
      gap: '12px',
      '@media (max-width: 639px)': {
        display: 'flex',
        marginX: '-20px',
        paddingX: '20px',
        paddingBottom: '4px',
        overflowX: 'auto',
        scrollSnapType: '[x mandatory]',
        scrollPaddingX: '20px',
        scrollbarWidth: 'none',
        '&::-webkit-scrollbar': { display: 'none' },
        '& > *': { flex: '[0 0 82%]', scrollSnapAlign: 'start' },
      },
    })}
  >
    {#each publications as publication (publication.id)}
      <PinnedCard publicationView$key={publication} />
    {/each}
  </div>
</section>
