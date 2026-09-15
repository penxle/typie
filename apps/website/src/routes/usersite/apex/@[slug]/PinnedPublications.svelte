<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { entryGroupCount, entryGroupLabel } from './entry-group-styles';
  import PinnedRow from './PinnedRow.svelte';
  import type { UsersiteSpace_PinnedRow_publicationView$key } from '$mearie';

  type Props = {
    publications: readonly ({ id: string } & UsersiteSpace_PinnedRow_publicationView$key)[];
  };

  let { publications }: Props = $props();
</script>

<section>
  <h2 class={css(entryGroupLabel)}>
    고정 <span class={css(entryGroupCount)}>{publications.length}</span>
  </h2>

  <div class={flex({ flexDirection: 'column', minWidth: '0' })}>
    {#each publications as publication (publication.id)}
      <PinnedRow publicationView$key={publication} />
    {/each}
  </div>
</section>
