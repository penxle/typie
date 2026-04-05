<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
  import Trash2Icon from '~icons/lucide/trash-2';
  import Undo2Icon from '~icons/lucide/undo-2';
  import { graphql } from '$mearie';
  import TrashEntity from './TrashEntity.svelte';
  import type { DashboardLayout_TrashTree_TrashFolder_folder$key } from '$mearie';

  type Props = {
    folder$key: DashboardLayout_TrashTree_TrashFolder_folder$key;
    onChange?: () => void;
  };

  let { folder$key, onChange }: Props = $props();

  const folder = createFragment(
    graphql(`
      fragment DashboardLayout_TrashTree_TrashFolder_folder on Folder {
        id
        name

        entity {
          id
          slug
          order
          depth

          ancestors {
            id
            node {
              __typename
              ... on Folder {
                id
                name
              }
            }
          }
        }
      }
    `),
    () => folder$key,
  );

  const [recoverEntity] = createMutation(
    graphql(`
      mutation DashboardLayout_TrashTree_TrashFolder_RecoverEntity_Mutation($input: RecoverEntityInput!) {
        recoverEntity(input: $input) {
          id

          state
          ancestors {
            id
            node {
              __typename
              ... on Folder {
                id
                name
              }
            }
          }
          node {
            __typename
            ... on Folder {
              id
              name
            }
            ... on Document {
              id
              title
            }
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

              children {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }
        }
      }
    `),
  );

  const [purgeEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_TrashTree_TrashFolder_PurgeEntities_Mutation($input: PurgeEntitiesInput!) {
        purgeEntities(input: $input) {
          id
        }
      }
    `),
  );

  let detailsEl = $state<HTMLDetailsElement>();
  let open = $state(false);

  const children = createQuery(
    graphql(`
      query DashboardLayout_TrashTree_TrashFolderChildren_Query($entityId: ID!) {
        entity(entityId: $entityId) {
          id

          deletedChildren {
            id
            deletedAt
            node {
              __typename
            }

            ...DashboardLayout_TrashTree_TrashEntity_entity
          }
        }
      }
    `),
    () => ({ entityId: folder.data.entity.id }),
    () => ({ skip: !open }),
  );

  const childEntities = $derived(children.data?.entity?.deletedChildren ?? []);
  const showLoading = $derived(children.loading && childEntities.length === 0);
</script>

<details
  bind:this={detailsEl}
  data-id={folder.data.entity.id}
  data-order={folder.data.entity.order}
  data-path-depth={folder.data.entity.depth}
  data-type="folder"
  bind:open
>
  <summary
    class={cx(
      'group',
      css({
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'space-between',
        gap: '6px',
        paddingX: '12px',
        paddingY: '6px',
        borderRadius: '8px',
        transition: 'common',
        cursor: 'pointer',
        _supportHover: { backgroundColor: 'surface.muted' },
      }),
    )}
    aria-selected="false"
    data-anchor={(children.data?.entity?.deletedChildren?.length ?? 0) > 0}
    onkeyup={(e) => {
      if (e.code === 'Space') {
        e.preventDefault();
      }
    }}
    role="treeitem"
  >
    <div class={css({ display: 'flex', alignItems: 'center', gap: '8px', minWidth: '0', flexGrow: '1' })}>
      <Icon style={css.raw({ color: 'text.faint', flexShrink: '0' })} icon={open ? ChevronDownIcon : ChevronRightIcon} size={14} />
      <Icon style={css.raw({ color: 'text.faint', flexShrink: '0' })} icon={FolderIcon} size={14} />

      <span
        class={css({
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          wordBreak: 'break-all',
          lineClamp: '1',
          flexShrink: '0',
          maxWidth: '[60%]',
        })}
      >
        {folder.data.name}
      </span>
    </div>

    <div
      class={css({
        display: 'flex',
        alignItems: 'center',
        gap: '2px',
        opacity: '0',
        transition: 'common',
        _groupHover: { opacity: '100' },
      })}
    >
      <button
        class={center({
          borderRadius: '4px',
          size: '24px',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        onclick={async () => {
          try {
            const resp = await recoverEntity({ input: { entityId: folder.data.entity.id } });
            const currentName =
              resp.recoverEntity.node.__typename === 'Folder'
                ? resp.recoverEntity.node.name
                : resp.recoverEntity.node.__typename === 'Document'
                  ? resp.recoverEntity.node.title
                  : '';
            const path = [
              ...resp.recoverEntity.ancestors
                .map((ancestor) => (ancestor.node.__typename === 'Folder' ? ancestor.node.name : ''))
                .filter((name) => name.length > 0),
              currentName,
            ]
              .filter((segment) => segment.length > 0)
              .join(' › ');

            onChange?.();
            mixpanel.track('recover_entity', { via: 'trash', type: 'folder' });
            Toast.success(`"${path}" 폴더를 복원했어요`);
          } catch {
            Toast.error('폴더 복원에 실패했어요');
          }
        }}
        type="button"
        use:tooltip={{ message: '복원', placement: 'top' }}
      >
        <Icon icon={Undo2Icon} size={14} />
      </button>
      <button
        class={center({
          borderRadius: '4px',
          size: '24px',
          color: 'text.subtle',
          transition: 'common',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        onclick={() => {
          Dialog.confirm({
            title: '폴더 영구 삭제',
            message: '영구 삭제한 폴더는 복원할 수 없어요. 폴더 내부의 모든 콘텐츠도 함께 삭제돼요. 정말 삭제하시겠어요?',
            action: 'danger',
            actionLabel: '영구 삭제',
            actionHandler: async () => {
              try {
                await purgeEntities({ input: { entityIds: [folder.data.entity.id] } });
                onChange?.();
                mixpanel.track('purge_entity', { via: 'trash', type: 'folder' });
              } catch {
                Toast.error('폴더 영구 삭제에 실패했어요');
              }
            },
          });
        }}
        type="button"
        use:tooltip={{ message: '영구 삭제', placement: 'top' }}
      >
        <Icon icon={Trash2Icon} size={14} />
      </button>
    </div>
  </summary>

  <div class={flex({ flexDirection: 'column', borderLeftWidth: '1px', marginLeft: '24px' })} aria-hidden={!open} role="tree">
    {#if showLoading}
      <div
        class={css({
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          gap: '6px',
          paddingX: '12px',
          paddingY: '6px',
          borderRadius: '8px',
        })}
      >
        <div class={css({ display: 'flex', alignItems: 'center', gap: '8px', minWidth: '0', flexGrow: '1' })}>
          <RingSpinner style={css.raw({ size: '14px', color: 'text.disabled' })} />
          <span
            class={css({
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'text.disabled',
              lineClamp: '1',
            })}
          >
            불러오는 중...
          </span>
        </div>
      </div>
    {:else}
      {#each childEntities as entity (entity.id)}
        <TrashEntity entity$key={entity} onChange={() => children.refetch()} />
      {:else}
        <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
          폴더가 비어있어요
        </div>
      {/each}
    {/if}
  </div>
</details>
