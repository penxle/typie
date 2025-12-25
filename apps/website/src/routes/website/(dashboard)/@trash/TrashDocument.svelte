<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import FileIcon from '~icons/lucide/file';
  import Trash2Icon from '~icons/lucide/trash-2';
  import Undo2Icon from '~icons/lucide/undo-2';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_TrashTree_TrashDocument_document } from '$graphql';

  type Props = {
    $document: DashboardLayout_TrashTree_TrashDocument_document;
  };

  let { $document: _document }: Props = $props();

  const document = fragment(
    _document,
    graphql(`
      fragment DashboardLayout_TrashTree_TrashDocument_document on Document {
        id
        title

        entity {
          id
          slug
        }
      }
    `),
  );

  const recoverEntity = graphql(`
    mutation DashboardLayout_TrashTree_TrashDocument_RecoverEntity_Mutation($input: RecoverEntityInput!) {
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
    mutation DashboardLayout_TrashTree_TrashDocument_PurgeEntities_Mutation($input: PurgeEntitiesInput!) {
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
    <Icon style={css.raw({ color: 'text.faint' })} icon={FileIcon} size={14} />

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
      {$document.title}
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
          await recoverEntity({ entityId: $document.entity.id });
          mixpanel.track('recover_entity', { via: 'trash', type: 'document' });
        } catch {
          Toast.error('문서 복원에 실패했어요');
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
          title: '문서 영구 삭제',
          message: '영구 삭제한 문서는 복원할 수 없어요. 정말 삭제하시겠어요?',
          action: 'danger',
          actionLabel: '영구 삭제',
          actionHandler: async () => {
            try {
              await purgeEntities({ entityIds: [$document.entity.id] });
              mixpanel.track('purge_entity', { via: 'trash', type: 'document' });
              Toast.success('문서를 영구 삭제했어요');
            } catch {
              Toast.error('문서 영구 삭제에 실패했어요');
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
