<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { graphql } from '$mearie';
  import TrashEntity from './TrashEntity.svelte';
  import type { DashboardLayout_TrashTree_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_TrashTree_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_TrashTree_site on Site {
        id

        deletedEntities {
          id
          node {
            __typename
          }
          ...DashboardLayout_TrashTree_TrashEntity_entity

          deletedChildren {
            id
            node {
              __typename
            }
            ...DashboardLayout_TrashTree_TrashEntity_entity

            deletedChildren {
              id
              node {
                __typename
              }
              ...DashboardLayout_TrashTree_TrashEntity_entity

              deletedChildren {
                id
                node {
                  __typename
                }
                ...DashboardLayout_TrashTree_TrashEntity_entity
              }
            }
          }
        }
      }
    `),
    () => site$key,
  );
</script>

<div
  class={flex({
    flexDirection: 'column',
    flexGrow: '1',
    height: 'full',
    userSelect: 'none',
  })}
  role="tree"
>
  {#if site.data.deletedEntities.length > 0}
    <div
      class={flex({
        flexDirection: 'column',
        flexGrow: '1',
        paddingX: '8px',
        paddingTop: '8px',
        paddingBottom: '32px',
        overflowY: 'auto',
      })}
    >
      {#each site.data.deletedEntities as entity (entity.id)}
        <TrashEntity entity$key={entity} />
      {/each}
    </div>
  {:else}
    <div class={center({ flexGrow: '1', flexDirection: 'column', gap: '6px' })}>
      <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>휴지통이 비어있어요.</p>
      <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>삭제 후 30일동안 휴지통에 보관돼요.</p>
    </div>
  {/if}
</div>
