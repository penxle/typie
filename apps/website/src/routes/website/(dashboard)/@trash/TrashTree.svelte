<script lang="ts">
  import TrashIcon from '~icons/lucide/trash';
  import { fragment, graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
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
  {#each $site.deletedEntities as entity (entity.id)}
    <TrashEntity $entity={entity} />
  {:else}
    <div class={center({ flexGrow: '1' })}>
      <div
        class={flex({
          flexDirection: 'column',
          alignItems: 'center',
          gap: '16px',
        })}
      >
        <Icon style={css.raw({ color: 'text.disabled' })} icon={TrashIcon} size={32} />
        <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>휴지통이 비어있어요</p>
      </div>
    </div>
  {/each}
</div>
