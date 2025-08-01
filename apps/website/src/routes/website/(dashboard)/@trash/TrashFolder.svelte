<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import FolderIcon from '~icons/lucide/folder';
  import Trash2Icon from '~icons/lucide/trash-2';
  import Undo2Icon from '~icons/lucide/undo-2';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { Dialog, Toast } from '$lib/notification';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import TrashEntity from './TrashEntity.svelte';
  import type { DashboardLayout_TrashTree_TrashFolder_entity, DashboardLayout_TrashTree_TrashFolder_folder, List } from '$graphql';

  type Props = {
    $folder: DashboardLayout_TrashTree_TrashFolder_folder;
    $entities: List<DashboardLayout_TrashTree_TrashFolder_entity>;
  };

  let { $folder: _folder, $entities: _entities }: Props = $props();

  const folder = fragment(
    _folder,
    graphql(`
      fragment DashboardLayout_TrashTree_TrashFolder_folder on Folder {
        id
        name

        entity {
          id
          slug
          order
          depth
        }
      }
    `),
  );

  const recoverEntity = graphql(`
    mutation DashboardLayout_TrashTree_TrashFolder_RecoverEntity_Mutation($input: RecoverEntityInput!) {
      recoverEntity(input: $input) {
        id

        state

        site {
          id
          ...DashboardLayout_Trash_site
        }
      }
    }
  `);

  const purgeEntities = graphql(`
    mutation DashboardLayout_TrashTree_TrashFolder_PurgeEntities_Mutation($input: PurgeEntitiesInput!) {
      purgeEntities(input: $input) {
        id
        ...DashboardLayout_Trash_site
      }
    }
  `);
  const entities = fragment(
    _entities,
    graphql(`
      fragment DashboardLayout_TrashTree_TrashFolder_entity on Entity {
        id

        ...DashboardLayout_TrashTree_TrashEntity_entity
      }
    `),
  );

  let detailsEl = $state<HTMLDetailsElement>();
  let open = $state(false);
</script>

<details
  bind:this={detailsEl}
  data-id={$folder.entity.id}
  data-order={$folder.entity.order}
  data-path-depth={$folder.entity.depth}
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
        paddingX: '8px',
        paddingY: '2px',
        borderRadius: '6px',
        transition: 'common',
        cursor: 'pointer',
        _supportHover: { backgroundColor: 'surface.muted' },
        '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
      }),
    )}
    aria-selected="false"
    data-anchor={$entities && $entities.length > 0}
    onkeyup={(e) => {
      if (e.code === 'Space') {
        e.preventDefault();
      }
    }}
    role="treeitem"
  >
    <div class={css({ display: 'flex', alignItems: 'center', gap: '6px', paddingY: '4px' })}>
      <Icon style={css.raw({ color: 'text.faint' })} icon={open ? ChevronDownIcon : ChevronRightIcon} size={14} />
      <Icon style={css.raw({ color: 'text.faint' })} icon={FolderIcon} size={14} />
      <span
        class={css({
          flexGrow: '1',
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.muted',
          wordBreak: 'break-all',
          lineClamp: '1',
        })}
      >
        {$folder.name}
      </span>
    </div>

    <div class={css({ display: 'flex', alignItems: 'center', gap: '4px' })}>
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
            await recoverEntity({ entityId: $folder.entity.id });
            mixpanel.track('recover_entity', { via: 'trash', type: 'folder' });
            Toast.success('폴더를 복원했어요');
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
                await purgeEntities({ entityIds: [$folder.entity.id] });
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
    {#each $entities as entity (entity.id)}
      <TrashEntity $entity={entity} />
    {:else}
      <div class={css({ paddingX: '8px', paddingY: '6px', fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>
        폴더가 비어있어요
      </div>
    {/each}
  </div>
</details>
