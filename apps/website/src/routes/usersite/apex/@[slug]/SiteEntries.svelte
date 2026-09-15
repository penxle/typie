<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { discoveryCardList } from '../(site)/discovery-styles';
  import DiscoveryCard from '../(site)/DiscoveryCard.svelte';
  import { entryGroupCount, entryGroupLabel } from './entry-group-styles';
  import FolderTile from './FolderTile.svelte';
  import type { UsersiteApex_DiscoveryCard_publicationView$key, UsersiteSpace_FolderTile_folderView$key } from '$mearie';

  type Entry =
    | ({ __typename: 'PublicationView'; id: string } & UsersiteApex_DiscoveryCard_publicationView$key)
    | ({ __typename: 'SiteFolderView'; id: string } & UsersiteSpace_FolderTile_folderView$key);

  type Props = {
    entries: readonly Entry[];
    nested?: boolean;
  };

  let { entries, nested = false }: Props = $props();

  const folders = $derived(entries.filter((entry) => entry.__typename === 'SiteFolderView'));
  const publications = $derived(entries.filter((entry) => entry.__typename === 'PublicationView'));
</script>

{#if folders.length > 0}
  <section class={css({ marginBottom: publications.length > 0 ? '44px' : '0' })}>
    <h2 class={css(entryGroupLabel)}>
      {nested ? '하위 시리즈' : '시리즈'}
      <span class={css(entryGroupCount)}>{folders.length}</span>
    </h2>

    <div class={css({ display: 'flex', flexDirection: 'column', minWidth: '0' })}>
      {#each folders as folder (folder.id)}
        <FolderTile folderView$key={folder} />
      {/each}
    </div>
  </section>
{/if}

{#if publications.length > 0}
  <section>
    <h2 class={css(entryGroupLabel)}>
      글 <span class={css(entryGroupCount)}>{publications.length}</span>
    </h2>

    <div class={css(discoveryCardList)}>
      {#each publications as publication (publication.id)}
        <DiscoveryCard context="space" publicationView$key={publication} showFolder={false} />
      {/each}
    </div>
  </section>
{/if}
