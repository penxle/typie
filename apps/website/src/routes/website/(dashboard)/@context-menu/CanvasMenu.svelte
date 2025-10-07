<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import Columns2Icon from '~icons/lucide/columns-2';
  import InfoIcon from '~icons/lucide/info';
  import Rows2Icon from '~icons/lucide/rows-2';
  import TrashIcon from '~icons/lucide/trash';
  import { graphql } from '$graphql';
  import { getSplitViewContext, getViewContext } from '../[slug]/@split-view/context.svelte';

  type Props = {
    canvas: {
      id: string;
      title: string;
    };
    entity: {
      slug: string;
    };
    via: 'tree' | 'editor';
  };

  let { canvas, entity, via }: Props = $props();

  const splitView = getSplitViewContext();
  const view = getViewContext();

  const handleAddSplitView = (direction: 'horizontal' | 'vertical') => {
    if (view) {
      splitView.addView(entity.slug, {
        viewId: view.id,
        direction,
        position: 'after',
      });
    } else {
      splitView.addViewAtRoot(entity.slug, direction);
    }
    mixpanel.track('add_split_view', { via, direction });
  };

  const deleteCanvas = graphql(`
    mutation CanvasMenu_DeleteCanvas_Mutation($input: DeleteCanvasInput!) {
      deleteCanvas(input: $input) {
        id

        entity {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_Trash_site
            ...DashboardLayout_PlanUsageWidget_site
          }
        }
      }
    }
  `);

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

<MenuItem icon={Columns2Icon} onclick={() => handleAddSplitView('horizontal')}>오른쪽에 열기</MenuItem>
<MenuItem icon={Rows2Icon} onclick={() => handleAddSplitView('vertical')}>아래에 열기</MenuItem>
<HorizontalDivider color="secondary" />

<MenuItem icon={TrashIcon} onclick={handleDelete} variant="danger">삭제</MenuItem>
