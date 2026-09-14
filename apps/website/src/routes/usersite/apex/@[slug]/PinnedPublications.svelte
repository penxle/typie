<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import PinIcon from '~icons/lucide/pin';
  import PinnedTile from './PinnedTile.svelte';
  import type { SpaceDateDisplay, UsersiteSpace_PinnedTile_publicationView$key } from '$mearie';

  type Props = {
    publications: readonly ({ id: string } & UsersiteSpace_PinnedTile_publicationView$key)[];
    dateDisplay: SpaceDateDisplay;
  };

  let { publications, dateDisplay }: Props = $props();
</script>

<section>
  <div class={flex({ alignItems: 'center', gap: '4px', fontSize: '12px', fontWeight: 'medium', color: 'text.hint' })}>
    <Icon icon={PinIcon} size={12} />
    <span>고정 글</span>
  </div>

  <div
    class={css({
      display: 'grid',
      gridTemplateColumns: { base: '1fr', lg: 'repeat(3, 1fr)' },
      gap: { base: '8px', lg: '16px' },
      marginTop: '10px',
    })}
  >
    {#each publications as publication (publication.id)}
      <PinnedTile {dateDisplay} publicationView$key={publication} />
    {/each}
  </div>
</section>
