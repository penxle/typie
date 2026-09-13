<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import SeriesCard from './SeriesCard.svelte';
  import type { UsersiteSpace_SeriesCard_collectionView$key } from '$mearie';

  type Props = {
    collections: readonly ({ id: string } & UsersiteSpace_SeriesCard_collectionView$key)[];
  };

  let { collections }: Props = $props();
</script>

{#if collections.length > 0}
  <div>
    {#each collections as collection (collection.id)}
      <SeriesCard collectionView$key={collection} />
    {/each}
  </div>
{:else}
  <div class={css({ paddingY: '80px', textAlign: 'center', fontSize: '14px', color: 'text.hint' })}>아직 만든 시리즈가 없어요</div>
{/if}
