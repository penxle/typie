<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import InfoIcon from '~icons/lucide/info';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { fragment, graphql } from '$graphql';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { Dialog, Toast } from '$lib/notification';
  import { clamp } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import TrashTree from './TrashTree.svelte';
  import type { DashboardLayout_Trash_site } from '$graphql';

  const app = getAppContext();

  type Props = {
    $site: DashboardLayout_Trash_site;
    containerElement?: HTMLElement | null;
  };

  let { $site: _site, containerElement = null }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_Trash_site on Site {
        id
        deletedEntities {
          id
        }
        ...DashboardLayout_TrashTree_site
      }
    `),
  );

  const purgeEntities = graphql(`
    mutation DashboardLayout_Trash_PurgeEntities($input: PurgeEntitiesInput!) {
      purgeEntities(input: $input) {
        id

        ...DashboardLayout_Trash_site
      }
    }
  `);

  type Resizer = {
    deltaY: number;
    eligible: boolean;
    event: PointerEvent;
    element: HTMLElement;
  };

  let resizer = $state<Resizer | null>(null);
  let containerHeight = $state(0);
  let maxHeight = $derived(containerHeight > 0 ? containerHeight / 2 : 300);
  let newHeight = $derived(clamp((app.preference.current.trashHeight ?? 300) + (resizer?.deltaY ?? 0), 200, maxHeight));

  $effect(() => {
    if (containerElement) {
      const observer = new ResizeObserver((entries) => {
        for (const entry of entries) {
          containerHeight = entry.contentRect.height;
        }
      });

      observer.observe(containerElement);
      containerHeight = containerElement.clientHeight;

      return () => observer.disconnect();
    }
  });
</script>

<div
  class={css({
    position: 'relative',
    borderTopWidth: '1px',
    borderColor: 'border.default',
    backgroundColor: 'surface.default',
  })}
  data-type="trash"
>
  {#if app.state.trashOpen}
    <div
      class={css({
        position: 'absolute',
        top: '-6px',
        left: '0',
        right: '0',
        zIndex: '2',
        height: '12px',
        cursor: 'row-resize',
        _hoverAfter: {
          content: '""',
          display: 'block',
          borderTopRadius: '4px',
          marginTop: '4px',
          width: 'full',
          height: '2px',
          backgroundColor: 'border.strong',
          opacity: '50',
        },
      })}
      onpointerdowncapture={(e) => {
        resizer = {
          element: e.currentTarget,
          event: e,
          deltaY: 0,
          eligible: false,
        };
      }}
      onpointermovecapture={(e) => {
        if (!resizer) return;

        if (!resizer.eligible) {
          resizer.eligible = true;
          resizer.element.setPointerCapture(e.pointerId);
        }

        resizer.deltaY = -Math.round(e.clientY - resizer.event.clientY);
      }}
      onpointerupcapture={() => {
        if (!resizer) return;

        if (resizer.eligible && resizer.element.hasPointerCapture(resizer.event.pointerId)) {
          resizer.element.releasePointerCapture(resizer.event.pointerId);
        }

        app.preference.current.trashHeight = newHeight;

        resizer = null;
      }}
    ></div>
  {/if}

  <div class={flex({ position: 'relative' })}>
    <button
      class={flex({
        alignItems: 'center',
        paddingX: '10px',
        paddingY: '6px',
        gap: '4px',
        flexGrow: '1',
        fontSize: '12px',
        fontWeight: 'medium',
        color: 'text.subtle',
        transition: 'common',
        cursor: 'pointer',
        userSelect: 'none',
        _hover: {
          backgroundColor: 'surface.subtle',
        },
      })}
      onclick={() => (app.state.trashOpen = !app.state.trashOpen)}
      type="button"
    >
      <Icon icon={app.state.trashOpen ? ChevronRightIcon : ChevronDownIcon} size={16} />
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <Icon icon={Trash2Icon} size={12} />
        <span class={css({ flexGrow: '1', textAlign: 'left' })}>휴지통</span>
        <div use:tooltip={{ message: '삭제 후 30일 동안 보관돼요', placement: 'top' }}>
          <Icon icon={InfoIcon} size={12} />
        </div>
      </div>
    </button>
    {#if app.state.trashOpen}
      <button
        class={center({
          position: 'absolute',
          top: '0',
          bottom: '0',
          marginY: 'auto',
          right: '10px',
          borderRadius: '4px',
          height: '24px',
          paddingX: '8px',
          paddingY: '4px',
          color: 'text.faint',
          transition: 'common',
          fontSize: '12px',
          fontWeight: 'medium',
          userSelect: 'none',
          _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
        })}
        onclick={(e: MouseEvent) => {
          e.stopPropagation();
          const entityIds = $site.deletedEntities.map((entity) => entity.id);
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
                await purgeEntities({ entityIds });
                mixpanel.track('purge_all_entities', { via: 'trash', count: entityIds.length });
                Toast.success('휴지통을 비웠어요');
              } catch {
                Toast.error('휴지통 비우기에 실패했어요');
              }
            },
          });
        }}
        type="button"
      >
        비우기
      </button>
    {/if}
  </div>

  {#if app.state.trashOpen}
    <div
      style:--height={`${newHeight}px`}
      class={css({
        display: 'flex',
        flexDirection: 'column',
        height: 'var(--height)',
        borderTopWidth: '1px',
        borderColor: 'border.subtle',
        backgroundColor: 'surface.subtle',
        overflow: 'hidden',
      })}
    >
      <div
        class={flex({
          flexDirection: 'column',
          flexGrow: '1',
          paddingX: '8px',
          paddingTop: '8px',
          paddingBottom: '32px',
          overflowY: 'auto',
        })}
      >
        <!-- <p
          class={css({
            fontSize: '12px',
            fontWeight: 'medium',
            color: 'text.disabled',
            display: 'flex',
            alignItems: 'center',
            gap: '2px',
            paddingX: '8px',
            paddingBottom: '4px',
          })}
        >
          <Icon style={css.raw({ color: 'text.disabled' })} icon={TriangleAlertIcon} size={12} />
          <span>삭제 후 30일 동안 보관돼요</span>
        </p> -->
        <TrashTree {$site} />
      </div>
    </div>
  {/if}
</div>
