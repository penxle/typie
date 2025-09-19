<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { clamp } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { sineInOut } from 'svelte/easing';
  import { fade } from 'svelte/transition';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { fragment, graphql } from '$graphql';
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
        paddingX: '12px',
        paddingY: '8px',
        gap: '4px',
        flexGrow: '1',
        color: 'text.faint',
        transition: 'common',
        transitionProperty: '[background-color]',
        cursor: 'pointer',
        userSelect: 'none',
        _hover: {
          backgroundColor: 'surface.subtle',
        },
      })}
      onclick={() => {
        app.state.trashOpen = !app.state.trashOpen;
        mixpanel.track('toggle_trash', { via: 'trash', open: app.state.trashOpen });
      }}
      type="button"
    >
      <Icon
        style={css.raw({
          transitionProperty: '[transform]',
          transform: app.state.trashOpen ? 'rotate(90deg)' : 'rotate(0deg)',
        })}
        icon={ChevronRightIcon}
        size={16}
      />

      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <Icon icon={Trash2Icon} size={12} />
        <span class={css({ flexGrow: '1', textAlign: 'left', fontSize: '13px', fontWeight: 'medium' })}>휴지통</span>
      </div>
    </button>

    {#if app.state.trashOpen}
      <button
        class={center({
          position: 'absolute',
          top: '0',
          bottom: '0',
          marginY: 'auto',
          right: '12px',
          borderRadius: '4px',
          height: '24px',
          paddingX: '8px',
          paddingY: '4px',
          color: 'text.faint',
          fontSize: '13px',
          fontWeight: 'medium',
          userSelect: 'none',
          transition: 'common',
          transitionProperty: '[color, background-color]',
          _hover: { backgroundColor: 'surface.muted', color: 'text.subtle' },
        })}
        onclick={(e) => {
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
                mixpanel.track('empty_trash', { via: 'trash', count: entityIds.length });
                Toast.success('휴지통을 비웠어요');
              } catch {
                Toast.error('휴지통 비우기에 실패했어요');
              }
            },
          });
        }}
        type="button"
        transition:fade={{ duration: 150, easing: sineInOut }}
      >
        비우기
      </button>
    {/if}
  </div>

  <div
    style:--height={`${app.state.trashOpen ? newHeight : 0}px`}
    class={css({
      display: 'flex',
      flexDirection: 'column',
      height: 'var(--height)',
      borderTopWidth: '1px',
      borderColor: 'border.subtle',
      backgroundColor: 'surface.default',
      overflowY: 'auto',
      opacity: app.state.trashOpen ? '100' : '0',
      transitionDuration: '150ms',
      transitionTimingFunction: 'ease',
      transitionProperty: resizer ? '[none]' : '[height, opacity]',
    })}
  >
    <TrashTree {$site} />
  </div>
</div>
