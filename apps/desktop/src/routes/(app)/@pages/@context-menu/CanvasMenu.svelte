<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import CopyIcon from '~icons/lucide/copy';
  import InfoIcon from '~icons/lucide/info';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { graphql } from '$graphql';

  type Props = {
    canvas: {
      id: string;
      title: string;
    };
    via: 'tree' | 'editor';
  };

  let { canvas, via }: Props = $props();

  const duplicateCanvas = graphql(`
    mutation CanvasMenu_DuplicateCanvas_Mutation($input: DuplicateCanvasInput!) {
      duplicateCanvas(input: $input) {
        id

        entity {
          id
          slug
        }
      }
    }
  `);

  const deleteCanvas = graphql(`
    mutation CanvasMenu_DeleteCanvas_Mutation($input: DeleteCanvasInput!) {
      deleteCanvas(input: $input) {
        id

        entity {
          id

          site {
            id
          }
        }
      }
    }
  `);

  const handleDuplicate = async () => {
    const resp = await duplicateCanvas({ canvasId: canvas.id });
    mixpanel.track('duplicate_canvas', { via });
    await goto(`/${resp.entity.slug}`);
  };

  const handleDelete = () => {
    Dialog.confirm({
      title: '캔버스 삭제',
      message: `정말 "${canvas.title}" 캔버스를 삭제하시겠어요?`,
      children: deleteDetailsView,
      action: 'danger',
      actionLabel: '삭제',
      actionHandler: async () => {
        await deleteCanvas({ canvasId: canvas.id });
        mixpanel.track('delete_canvas', { via });
      },
    });
  };
</script>

{#snippet deleteDetailsView()}
  <div
    class={flex({
      alignItems: 'center',
      gap: '6px',
      borderRadius: '8px',
      paddingX: '12px',
      paddingY: '8px',
      backgroundColor: 'surface.subtle',
    })}
  >
    <Icon style={css.raw({ color: 'text.muted' })} icon={InfoIcon} size={14} />
    <span class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.muted' })}>삭제 후 30일 동안 휴지통에 보관돼요</span>
  </div>
{/snippet}

<MenuItem icon={CopyIcon} onclick={handleDuplicate}>복제</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem icon={TrashIcon} onclick={handleDelete} variant="danger">삭제</MenuItem>
