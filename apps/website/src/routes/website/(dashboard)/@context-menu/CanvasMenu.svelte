<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, MenuItem } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import Columns2Icon from '~icons/lucide/columns-2';
  import CopyIcon from '~icons/lucide/copy';
  import InfoIcon from '~icons/lucide/info';
  import Rows2Icon from '~icons/lucide/rows-2';
  import TrashIcon from '~icons/lucide/trash';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { getSplitViewContext } from '../[slug]/@split-view/context.svelte';
  import { addSplitView } from '../[slug]/@split-view/utils';

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

  const app = getAppContext();
  const splitView = getSplitViewContext();

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
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_Trash_site
            ...DashboardLayout_PlanUsageWidget_site
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

  const handleAddSplitView = (direction: 'horizontal' | 'vertical') => {
    if (page.params.slug && splitView.state.current.view) {
      const { splitViews, focusedSplitViewId } = addSplitView(splitView.state.current.view, entity.slug, direction);
      splitView.state.current.view = splitViews;
      splitView.state.current.focusedViewId = focusedSplitViewId;
      mixpanel.track('add_split_view', { via, direction });
    }
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

{#if app.preference.current.experimental_splitViewEnabled}
  <MenuItem icon={Columns2Icon} onclick={() => handleAddSplitView('horizontal')}>오른쪽에 열기</MenuItem>
  <MenuItem icon={Rows2Icon} onclick={() => handleAddSplitView('vertical')}>아래에 열기</MenuItem>
  <HorizontalDivider color="secondary" />
{/if}

<MenuItem icon={CopyIcon} onclick={handleDuplicate}>복제</MenuItem>

<HorizontalDivider color="secondary" />

<MenuItem icon={TrashIcon} onclick={handleDelete} variant="danger">삭제</MenuItem>
