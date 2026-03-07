<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$mearie';
  import TrashTree from './TrashTree.svelte';
  import type { DashboardLayout_TrashModal_site$key } from '$mearie';

  type Props = {
    site$key: DashboardLayout_TrashModal_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_TrashModal_site on Site {
        id
        deletedEntities {
          id
        }
        ...DashboardLayout_TrashTree_site
      }
    `),
    () => site$key,
  );

  const [purgeEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_TrashModal_PurgeEntities($input: PurgeEntitiesInput!) {
        purgeEntities(input: $input) {
          id

          ...DashboardLayout_TrashModal_site
        }
      }
    `),
  );

  const app = getAppContext();

  const entityCount = $derived(site.data.deletedEntities.length);

  const handleEmptyTrash = async () => {
    const entityIds = site.data.deletedEntities.map((entity) => entity.id);
    if (entityIds.length === 0) {
      Toast.success('휴지통이 비어있어요');
      return;
    }

    Dialog.confirm({
      title: '휴지통 비우기',
      message: `휴지통에 있는 ${entityIds.length}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요.`,
      action: 'danger',
      actionLabel: '모두 삭제',
      actionHandler: async () => {
        try {
          await purgeEntities({ input: { entityIds } });
          mixpanel.track('empty_trash', { via: 'trash', count: entityIds.length });
          Toast.success('휴지통을 비웠어요');
        } catch {
          Toast.error('휴지통 비우기에 실패했어요');
        }
      },
    });
  };
</script>

<Modal
  style={css.raw({
    gap: '0',
    maxWidth: '520px',
    padding: '0',
  })}
  onclose={() => {
    app.state.trashOpen = false;
  }}
  open={app.state.trashOpen}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center', paddingX: '24px', paddingY: '20px' })}>
    <div class={flex({ alignItems: 'center', gap: '10px' })}>
      <Icon style={css.raw({ color: 'text.subtle' })} icon={Trash2Icon} size={20} />
      <span class={css({ fontSize: '18px', fontWeight: 'bold', color: 'text.subtle' })}>휴지통</span>
      {#if entityCount > 0}
        <span
          class={center({
            minWidth: '20px',
            height: '20px',
            paddingX: '6px',
            borderRadius: 'full',
            backgroundColor: 'surface.muted',
            fontSize: '12px',
            fontWeight: 'semibold',
            color: 'text.muted',
          })}
        >
          {entityCount}
        </span>
      {/if}
    </div>
    {#if entityCount > 0}
      <Button onclick={handleEmptyTrash} size="sm" variant="secondary">비우기</Button>
    {/if}
  </div>

  <HorizontalDivider />

  <div
    class={css({
      height: '400px',
      maxHeight: '[60vh]',
    })}
  >
    <TrashTree site$key={site.data} />
  </div>
</Modal>
