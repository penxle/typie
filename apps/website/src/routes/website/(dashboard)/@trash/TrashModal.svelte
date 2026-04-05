<script lang="ts">
  import { createFragment, createMutation, createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Modal } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$mearie';
  import TrashTree from './TrashTree.svelte';

  type Props = {
    siteId: string;
  };

  let { siteId }: Props = $props();

  const app = getAppContext();

  const siteFragment = graphql(`
    fragment DashboardLayout_TrashModal_site on Site {
      id
      deletedEntities {
        id
      }
      ...DashboardLayout_TrashTree_site
    }
  `);

  const query = createQuery(
    graphql(`
      query DashboardLayout_TrashModal_Query($siteId: ID!) {
        site(siteId: $siteId) {
          id
          ...DashboardLayout_TrashModal_site
        }
      }
    `),
    () => ({ siteId }),
    () => ({ skip: !app.state.trashOpen }),
  );

  const site = createFragment(siteFragment, () => query.data?.site);

  const [purgeEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_TrashModal_PurgeEntities($input: PurgeEntitiesInput!) {
        purgeEntities(input: $input) {
          id
        }
      }
    `),
  );

  const entityCount = $derived(site.data?.deletedEntities.length ?? 0);

  const handleEmptyTrash = async () => {
    const entityIds = site.data?.deletedEntities.map((entity) => entity.id) ?? [];
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
          await query.refetch();
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
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center', paddingX: '20px', paddingY: '16px' })}>
    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      <Icon style={css.raw({ color: 'text.subtle' })} icon={Trash2Icon} size={16} />
      <span class={css({ fontSize: '14px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.subtle' })}>휴지통</span>
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
    {#if site.data}
      <TrashTree loading={query.loading} onChange={() => query.refetch()} site$key={site.data} />
    {:else}
      <div class={center({ height: 'full' })}>
        <span class={css({ fontSize: '13px', color: 'text.disabled' })}>불러오는 중...</span>
      </div>
    {/if}
  </div>
</Modal>
