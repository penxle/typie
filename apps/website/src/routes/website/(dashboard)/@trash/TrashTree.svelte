<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { fragment, graphql } from '$graphql';
  import TrashEntity from './TrashEntity.svelte';
  import type { DashboardLayout_TrashTree_site } from '$graphql';

  type Props = {
    $site: DashboardLayout_TrashTree_site;
  };

  let { $site: _site }: Props = $props();

  const site = fragment(
    _site,
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
  );
</script>

<div
  class={flex({
    flexDirection: 'column',
    flexGrow: '1',
    userSelect: 'none',
  })}
  role="tree"
>
  {#if $site.deletedEntities.length > 0}
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
      {#each $site.deletedEntities as entity (entity.id)}
        <TrashEntity $entity={entity} />
      {/each}
    </div>
  {:else}
    <div class={center({ flexGrow: '1' })}>
      <p class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.disabled' })}>휴지통이 비어있어요</p>
    </div>
  {/if}
</div>
