<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Switch } from '@typie/ui/components';
  import { AdminEmpty } from '$lib/components/admin';
  import { graphql } from '$mearie';
  import SiteEntityTree from './SiteEntityTree.svelte';
  import type { AdminUserContentsTab_user$key } from '$mearie';

  type Props = {
    user$key: AdminUserContentsTab_user$key;
  };

  let { user$key }: Props = $props();

  let includeDeleted = $state(false);

  const user = createFragment(
    graphql(`
      fragment AdminUserContentsTab_user on User {
        id

        sites {
          id
          name
        }
      }
    `),
    () => user$key,
  );
</script>

<div class={flex({ alignItems: 'center', gap: '8px', marginBottom: '16px' })}>
  <Switch bind:checked={includeDeleted} />
  <span class={css({ fontSize: '13px', color: 'text.muted' })}>삭제된 항목 표시</span>
</div>

{#if user.data.sites.length === 0}
  <AdminEmpty text="사이트가 없습니다" />
{:else}
  {#each user.data.sites as site (site.id)}
    <div
      class={css({
        marginBottom: '16px',
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '12px',
        backgroundColor: 'admin.card.default',
        boxShadow: 'adminCard',
        padding: '16px',
      })}
    >
      <div class={css({ marginBottom: '10px', fontSize: '11px', fontWeight: 'semibold', letterSpacing: '[0.05em]', color: 'text.faint' })}>
        {site.name}
      </div>
      <SiteEntityTree {includeDeleted} siteId={site.id} />
    </div>
  {/each}
{/if}
