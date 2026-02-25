<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
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
    gap: '16px',
    maxWidth: '400px',
    padding: '24px',
  })}
  onclose={() => {
    app.state.trashOpen = false;
  }}
  open={app.state.trashOpen}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
    <div class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.subtle' })}>휴지통</div>
    <Button onclick={handleEmptyTrash} size="sm" variant="secondary">비우기</Button>
  </div>

  <div
    class={css({
      height: '400px',
      maxHeight: '[60vh]',
      overflowY: 'auto',
    })}
  >
    <TrashTree site$key={site.data} />
  </div>
</Modal>
