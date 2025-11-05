<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { PostType } from '@/enums';
  import FileIcon from '~icons/lucide/file';
  import ShapesIcon from '~icons/lucide/shapes';
  import Trash2Icon from '~icons/lucide/trash-2';
  import Undo2Icon from '~icons/lucide/undo-2';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_TrashTree_TrashPost_post } from '$graphql';

  type Props = {
    $post: DashboardLayout_TrashTree_TrashPost_post;
  };

  let { $post: _post }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment DashboardLayout_TrashTree_TrashPost_post on Post {
        id
        type
        title
        characterCount

        entity {
          id
          slug
        }
      }
    `),
  );

  const recoverEntity = graphql(`
    mutation DashboardLayout_TrashTree_TrashPost_RecoverEntity_Mutation($input: RecoverEntityInput!) {
      recoverEntity(input: $input) {
        id

        state

        site {
          id
          ...DashboardLayout_TrashModal_site
        }
      }
    }
  `);

  const purgeEntities = graphql(`
    mutation DashboardLayout_TrashTree_TrashPost_PurgeEntities_Mutation($input: PurgeEntitiesInput!) {
      purgeEntities(input: $input) {
        id
        ...DashboardLayout_TrashModal_site
      }
    }
  `);
</script>

<div
  class={css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: '6px',
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: '6px',
    transition: 'common',
    _hover: { backgroundColor: 'surface.muted' },
    '&:has([aria-pressed="true"])': { backgroundColor: 'surface.muted' },
  })}
  aria-selected="false"
  role="treeitem"
>
  <div class={css({ display: 'flex', alignItems: 'center', gap: '6px', paddingY: '4px' })}>
    {#if $post.type === PostType.NORMAL}
      <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} size={14} />
    {:else if $post.type === PostType.TEMPLATE}
      <Icon style={css.raw({ color: 'text.faint' })} icon={ShapesIcon} size={14} />
    {/if}

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
      {$post.title}
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
          await recoverEntity({ entityId: $post.entity.id });
          mixpanel.track('recover_entity', { via: 'trash', type: 'post' });
        } catch {
          Toast.error('포스트 복원에 실패했어요');
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
          title: '포스트 영구 삭제',
          message: '영구 삭제한 포스트는 복원할 수 없어요. 정말 삭제하시겠어요?',
          action: 'danger',
          actionLabel: '영구 삭제',
          actionHandler: async () => {
            try {
              await purgeEntities({ entityIds: [$post.entity.id] });
              mixpanel.track('purge_entity', { via: 'trash', type: 'post' });
              Toast.success('포스트를 영구 삭제했어요');
            } catch {
              Toast.error('포스트 영구 삭제에 실패했어요');
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
</div>
