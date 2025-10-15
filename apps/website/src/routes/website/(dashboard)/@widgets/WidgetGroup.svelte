<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip } from '@typie/ui/notification';
  import { animateFlip, createDndHandler, handleDragScroll } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import LayoutDashboardIcon from '~icons/lucide/layout-dashboard';
  import MinusIcon from '~icons/lucide/minus';
  import ShapesIcon from '~icons/lucide/shapes';
  import XIcon from '~icons/lucide/x';
  import { fragment, graphql } from '$graphql';
  import { getSplitViewContext } from '../[slug]/@split-view/context.svelte';
  import { getEditorRegistry } from '../[slug]/@split-view/editor-registry.svelte';
  import { findViewById } from '../[slug]/@split-view/utils';
  import { setupWidgetContext } from './widget-context.svelte';
  import WidgetPalette from './WidgetPalette.svelte';
  import { WIDGET_COMPONENTS } from './widgets';
  import type { WidgetGroup_query } from '$graphql';
  import type { WidgetPosition, WidgetType } from './widget-context.svelte';

  type Props = {
    $query: WidgetGroup_query;
  };

  let { $query: _query }: Props = $props();

  const query = fragment(
    _query,
    graphql(`
      fragment WidgetGroup_query on Query {
        widgets {
          id
          name
          data
          order
        }
      }
    `),
  );

  const postQuery = graphql(`
    query WidgetGroup_Query($slug: String!) @client {
      entity(slug: $slug) {
        id

        node {
          __typename

          ... on Post {
            id

            ...Editor_Widget_CharacterCountChangeWidget_post
            ...Editor_Widget_PostRelatedNoteWidget_post
          }
        }
      }
    }
  `);

  const createWidgetMutation = graphql(`
    mutation WidgetGroup_createWidget_Mutation($input: CreateWidgetInput!) {
      createWidget(input: $input) {
        id
        name
        data
        order
      }
    }
  `);

  const deleteWidgetMutation = graphql(`
    mutation WidgetGroup_deleteWidget_Mutation($input: DeleteWidgetInput!) {
      deleteWidget(input: $input) {
        id
      }
    }
  `);

  const moveWidgetMutation = graphql(`
    mutation WidgetGroup_moveWidget_Mutation($input: MoveWidgetInput!) {
      moveWidget(input: $input) {
        id
        order
      }
    }
  `);

  const updateWidgetMutation = graphql(`
    mutation WidgetGroup_updateWidget_Mutation($input: UpdateWidgetInput!) {
      updateWidget(input: $input) {
        id
        data
      }
    }
  `);

  const editorRegistry = getEditorRegistry();
  const splitView = getSplitViewContext();

  const focusedViewId = $derived(splitView.state.current.focusedViewId);

  const focusedView = $derived(
    focusedViewId && splitView.state.current.view ? findViewById(splitView.state.current.view, focusedViewId) : null,
  );

  const focusedViewSlug = $derived(focusedView?.type === 'item' ? focusedView.slug : null);
  const editor = $derived(focusedViewId && focusedViewSlug ? editorRegistry.get(focusedViewId, focusedViewSlug) : undefined);
  const _post = $derived(focusedViewSlug && $postQuery?.entity?.node?.__typename === 'Post' ? $postQuery.entity.node : undefined);

  $effect(() => {
    if (focusedViewSlug) {
      postQuery.load({ slug: focusedViewSlug });
    }
  });

  const app = getAppContext();

  let editMode = $state(false);
  let isHidden = $derived.by(() => app.preference.current.widgetHidden);
  let transitioning = $state(false);
  let altPressed = $state(false);

  const transformRight = $derived.by(() => {
    if (!isHidden || editMode) {
      return 'translateX(0)';
    }

    return 'translateX(calc(100% + 24px))';
  });

  let dropZoneElement = $state<HTMLDivElement>();
  let widgetListElement = $state<HTMLDivElement>();
  let scrollContainerElement = $state<HTMLDivElement>();
  let freePositionListElement = $state<HTMLDivElement>();

  type BaseDragging = {
    dropIndex: number | null;
    dropped?: boolean;
    isOutsideDropZone: boolean;
    cursorPosition: { x: number; y: number };
    widgetType: WidgetType;
    widgetData: Record<string, unknown>;
    calculatedPosition?: { top?: string; left?: string; bottom?: string; right?: string };
    widgetRect: { width: number; height: number };
    offsetX: number;
    offsetY: number;
  };

  type GroupDragging = BaseDragging & {
    source: 'group';
    widgetId: string;
  };

  type FreePositionDragging = BaseDragging & {
    source: 'freePosition';
    widgetId: string;
  };

  type PaletteDragging = BaseDragging & {
    source: 'palette';
  };

  type DraggingState = GroupDragging | FreePositionDragging | PaletteDragging;

  let dragging = $state<DraggingState | null>(null);

  const updateDropPosition = (e: PointerEvent) => {
    if (!dropZoneElement || !widgetListElement || !dragging) {
      return;
    }

    dragging.cursorPosition = { x: e.clientX, y: e.clientY };

    const dropZoneRect = dropZoneElement.getBoundingClientRect();
    const isInsideDropZone =
      e.clientX >= dropZoneRect.left &&
      e.clientX <= dropZoneRect.right &&
      e.clientY >= dropZoneRect.top &&
      e.clientY <= dropZoneRect.bottom;

    if (!isInsideDropZone) {
      dragging.dropIndex = null;
      dragging.isOutsideDropZone = true;
      return;
    }

    dragging.isOutsideDropZone = false;

    const widgetElements = [...widgetListElement.querySelectorAll('[data-widget-id]')] as HTMLElement[];
    let foundIndex: number | null = null;

    for (const [i, element] of widgetElements.entries()) {
      const rect = element.getBoundingClientRect();
      const midY = rect.top + rect.height / 2;

      if (e.clientY < midY) {
        foundIndex = i;
        break;
      }
    }

    if (foundIndex === null) {
      foundIndex = widgetElements.length;
    }

    dragging.dropIndex = foundIndex;
  };

  const calculateWidgetPosition = (
    cursorX: number,
    cursorY: number,
    widgetRect: { width: number; height: number },
    offsetX: number,
    offsetY: number,
  ) => {
    const widgetLeft = cursorX - offsetX;
    const widgetTop = cursorY - offsetY;

    const viewportWidth = window.innerWidth;
    const viewportHeight = window.innerHeight;

    const centerX = viewportWidth / 2;
    const centerY = viewportHeight / 2;

    const widgetCenterX = widgetLeft + widgetRect.width / 2;
    const widgetCenterY = widgetTop + widgetRect.height / 2;

    const horizontal = widgetCenterX < centerX ? 'left' : 'right';
    const vertical = widgetCenterY < centerY ? 'top' : 'bottom';

    const distanceToLeft = Math.max(0, widgetLeft);
    const distanceToRight = Math.max(0, viewportWidth - (widgetLeft + widgetRect.width));
    const distanceToTop = Math.max(0, widgetTop);
    const distanceToBottom = Math.max(0, viewportHeight - (widgetTop + widgetRect.height));

    const horizontalValue =
      horizontal === 'left'
        ? `${((distanceToLeft / viewportWidth) * 100).toFixed(2)}%`
        : `${((distanceToRight / viewportWidth) * 100).toFixed(2)}%`;
    const verticalValue =
      vertical === 'top'
        ? `${((distanceToTop / viewportHeight) * 100).toFixed(2)}%`
        : `${((distanceToBottom / viewportHeight) * 100).toFixed(2)}%`;

    return {
      [horizontal]: horizontalValue,
      [vertical]: verticalValue,
    };
  };

  const widgetContext = setupWidgetContext();

  widgetContext.createWidget = async (type: WidgetType, via: string, index?: number) => {
    const widgets = $query.widgets;
    let lowerOrder: string | undefined;
    let upperOrder: string | undefined;

    if (index === undefined || index >= widgets.length) {
      lowerOrder = widgets.at(-1)?.order;
      upperOrder = undefined;
    } else if (index === 0) {
      lowerOrder = undefined;
      upperOrder = widgets[0]?.order;
    } else {
      lowerOrder = widgets[index - 1]?.order;
      upperOrder = widgets[index]?.order;
    }

    try {
      await createWidgetMutation({
        name: type,
        data: {},
        lowerOrder,
        upperOrder,
      });

      mixpanel.track('create_widget', {
        widgetType: type,
        via,
      });
    } finally {
      await cache.invalidate({ __typename: 'Query', field: 'widgets' });
    }
  };

  widgetContext.deleteWidget = async (id: string, via: string) => {
    optimisticDeletedWidgetIds = [...optimisticDeletedWidgetIds, id];
    const widget = $query.widgets.find((w) => w.id === id);
    try {
      await deleteWidgetMutation({ widgetId: id });

      mixpanel.track('delete_widget', {
        widgetType: widget?.name,
        via,
      });
    } catch (err) {
      optimisticDeletedWidgetIds = optimisticDeletedWidgetIds.filter((existingId) => existingId !== id);
      throw err;
    } finally {
      await cache.invalidate({ __typename: 'Query', field: 'widgets' });
    }
  };

  widgetContext.updateWidget = async (widgetId: string, data: Record<string, unknown>) => {
    try {
      await updateWidgetMutation({ widgetId, data }, { optimistic: { id: widgetId, data } });
    } finally {
      cache.invalidate({ __typename: 'Query', field: 'widgets' });
    }
  };

  widgetContext.moveWidgetInGroup = async (widgetId: string, targetIndex: number) => {
    const widgets = $query.widgets;
    const widget = widgets.find((w) => w.id === widgetId);
    if (!widget) return;

    const wasAttaching = !!widget.data.position;

    if (wasAttaching) {
      optimisticDeletedWidgetIds = [...optimisticDeletedWidgetIds, widgetId];
    }

    try {
      const globalSorted = [...widgets]
        .toSorted((a, b) => a.order.localeCompare(b.order))
        .filter((w) => !optimisticDeletedWidgetIds.includes(w.id));

      const groupWidgetIds = globalSorted.filter((w) => !w.data.position && w.id !== widgetId).map((w) => w.id);

      const leftId = targetIndex === 0 ? undefined : groupWidgetIds[targetIndex - 1];
      const rightId = targetIndex >= groupWidgetIds.length ? undefined : groupWidgetIds[targetIndex];

      const lowerOrder = leftId ? globalSorted.find((w) => w.id === leftId)?.order : undefined;
      const upperOrder = rightId ? globalSorted.find((w) => w.id === rightId)?.order : undefined;

      await moveWidgetMutation({
        widgetId,
        lowerOrder,
        upperOrder,
      });

      if (wasAttaching) {
        await updateWidgetMutation(
          {
            widgetId,
            data: {
              ...widget.data,
              position: null,
            },
          },
          {
            optimistic: {
              id: widgetId,
              data: {
                ...widget.data,
                position: null,
              },
            },
          },
        );
      }

      if (wasAttaching) {
        mixpanel.track('attach_widget', {
          widgetType: widget.name,
        });
      } else {
        mixpanel.track('move_widget', {
          widgetType: widget.name,
        });
      }
    } finally {
      if (wasAttaching) {
        optimisticDeletedWidgetIds = optimisticDeletedWidgetIds.filter((id) => id !== widgetId);
      }
      await cache.invalidate({ __typename: 'Query', field: 'widgets' });
    }
  };

  widgetContext.moveWidgetToFreePosition = async (widgetId: string, position: WidgetPosition) => {
    const widgets = $query.widgets;
    const widget = widgets.find((w) => w.id === widgetId);
    if (!widget) return;

    const wasDetaching = !widget.data.position;

    const globalSorted = [...widgets]
      .toSorted((a, b) => a.order.localeCompare(b.order))
      .filter((w) => !optimisticDeletedWidgetIds.includes(w.id));

    const lowerOrder = globalSorted.at(-1)?.order;
    const upperOrder = undefined;

    try {
      await updateWidgetMutation(
        {
          widgetId,
          data: {
            ...widget.data,
            position,
          },
        },
        {
          optimistic: {
            id: widgetId,
            data: {
              ...widget.data,
              position,
            },
          },
        },
      );

      await moveWidgetMutation({
        widgetId,
        lowerOrder,
        upperOrder,
      });

      if (wasDetaching) {
        mixpanel.track('detach_widget', {
          widgetType: widget.name,
        });
      } else {
        mixpanel.track('move_widget', {
          widgetType: widget.name,
        });
      }
    } finally {
      await cache.invalidate({ __typename: 'Query', field: 'widgets' });
    }
  };

  type RealWidget = {
    type: 'real';
    id: string;
    name: string;
    data: Record<string, unknown>;
    order: string;
  };

  type PreviewWidget = {
    type: 'preview';
    id: string;
    widgetType: WidgetType;
    widgetData: Record<string, unknown>;
  };

  type DropareaWidget = {
    type: 'droparea';
    id: string;
    height: number;
  };

  type WidgetItem = RealWidget | PreviewWidget | DropareaWidget;

  let optimisticDeletedWidgetIds = $state<string[]>([]);
  let widgetsInGroup = $state<WidgetItem[]>([]);
  let freePositionWidgets = $state<RealWidget[]>([]);
  let prevWidgetCount = 0;

  $effect.pre(() => {
    const newCount = $query.widgets.length;

    if (newCount < prevWidgetCount) {
      optimisticDeletedWidgetIds = optimisticDeletedWidgetIds.filter((id) => $query.widgets.find((w) => w.id === id));
    }

    prevWidgetCount = newCount;

    const widgets = $query.widgets.filter(
      (w) =>
        !(dragging?.source === 'group' && dragging.widgetId === w.id) && !optimisticDeletedWidgetIds.includes(w.id) && !w.data.position,
    );
    const sorted = widgets.toSorted((a, b) => a.order.localeCompare(b.order));

    const result: WidgetItem[] = sorted.map((w) => ({ type: 'real' as const, ...w }));

    if (dragging?.widgetType && dragging.dropIndex !== null) {
      // NOTE: dragging이 아직 초기화되지 않았는데 optimistic이든 뭐든 위젯이 추가된 경우 drop preview와 동시에 보이지 않도록 함
      const { source, widgetType } = dragging;
      const widgetId = 'widgetId' in dragging ? dragging.widgetId : undefined;

      const isAlreadyInGroup = sorted.some((w) => {
        if (source === 'freePosition' && widgetId) {
          return w.id === widgetId;
        } else if (source === 'palette') {
          return w.name === widgetType;
        }
        return false;
      });

      if (!isAlreadyInGroup) {
        const previewWidget: PreviewWidget = {
          type: 'preview',
          id: 'drop-preview',
          widgetType: dragging.widgetType,
          widgetData: dragging.widgetData,
        };
        result.splice(dragging.dropIndex, 0, previewWidget);
      }
    }

    if (dragging?.dropIndex === null && !dragging.dropped) {
      result.unshift({
        type: 'droparea',
        id: 'droparea',
        height: dragging.widgetRect.height,
      } as DropareaWidget);
    }

    widgetsInGroup = result;

    freePositionWidgets = $query.widgets
      .filter((w) => w.data.position && !optimisticDeletedWidgetIds.includes(w.id))
      .map((w) => ({ type: 'real' as const, ...w }));
  });

  $effect(() => {
    widgetContext.env.editMode = editMode;
    widgetContext.env.editor = editor;
    widgetContext.env.$post = _post;
  });

  $effect(() => {
    if (!widgetListElement) return;

    const dndHandler = createDndHandler(widgetListElement, {
      dragHandleSelector: '[data-drag-handle]',
      getDragTarget: (e) => {
        const target = e.target as HTMLElement;
        return target.closest('[data-widget-id]') as HTMLElement;
      },
      canStartDrag: (e, widgetElement) => {
        const widgetId = widgetElement.dataset.widgetId;
        if (!widgetId) return false;

        e.preventDefault();
        return true;
      },
      onDragStart: (e, widgetElement) => {
        const widgetId = widgetElement.dataset.widgetId;
        if (!widgetId) return;

        const widget = $query.widgets.find((w) => w.id === widgetId);
        if (!widget) return;

        const rect = widgetElement.getBoundingClientRect();

        dragging = {
          dropIndex: $query.widgets.findIndex((w) => w.id === widgetId),
          isOutsideDropZone: false,
          cursorPosition: { x: e.clientX, y: e.clientY },
          source: 'group',
          widgetId,
          widgetType: widget.name as WidgetType,
          widgetData: widget.data,
          widgetRect: { width: rect.width, height: rect.height },
          offsetX: e.clientX - rect.left,
          offsetY: e.clientY - rect.top,
        };
      },
      onDragMove: (e) => {
        updateDropPosition(e);
      },
      onDragEnd: async (e) => {
        if (dragging && dragging.source === 'group' && dropZoneElement) {
          const currentDragging = dragging;
          currentDragging.dropped = true;

          try {
            if (currentDragging.isOutsideDropZone) {
              const { widgetId, widgetRect, offsetX, offsetY } = currentDragging;
              const widget = $query.widgets.find((w) => w.id === widgetId);
              if (widget) {
                const newPosition = calculateWidgetPosition(e.clientX, e.clientY, widgetRect, offsetX, offsetY);

                await widgetContext.moveWidgetToFreePosition?.(widgetId, newPosition);
              }
            } else {
              if (currentDragging.dropIndex !== null) {
                await widgetContext.moveWidgetInGroup?.(currentDragging.widgetId, currentDragging.dropIndex);
              }
            }
          } finally {
            if (dragging === currentDragging) {
              dragging = null;
            }
          }
        }
      },
      onDragCancel: () => {
        dragging = null;
      },
    });

    return () => {
      dndHandler.destroy();
    };
  });

  $effect(() => {
    if (!scrollContainerElement) return;
    return handleDragScroll(scrollContainerElement, !!dragging);
  });

  $effect(() => {
    if (!freePositionListElement) return;

    const dndHandler = createDndHandler(freePositionListElement, {
      dragHandleSelector: '[data-drag-handle]',
      showGhost: false,
      getDragTarget: (e) => {
        const target = e.target as HTMLElement;
        return target.closest('[data-free-positioned]') as HTMLElement;
      },
      canStartDrag: (e, widgetElement) => {
        const widgetId = widgetElement.dataset.widgetId;
        if (!widgetId) return false;

        e.preventDefault();
        return true;
      },
      onDragStart: async (e, widgetElement) => {
        const widgetId = widgetElement.dataset.widgetId;
        if (!widgetId) return;

        const widget = $query.widgets.find((w) => w.id === widgetId);
        if (!widget) return;

        const rect = widgetElement.getBoundingClientRect();
        const offsetX = e.clientX - rect.left;
        const offsetY = e.clientY - rect.top;

        dragging = {
          dropIndex: null,
          isOutsideDropZone: true,
          cursorPosition: { x: e.clientX, y: e.clientY },
          source: 'freePosition',
          widgetId,
          widgetType: widget.name as WidgetType,
          widgetData: widget.data,
          widgetRect: { width: rect.width, height: rect.height },
          offsetX,
          offsetY,
        };
      },
      onDragMove: (e) => {
        if (dragging && dragging.widgetRect && dragging.offsetX !== undefined && dragging.offsetY !== undefined) {
          dragging.cursorPosition = { x: e.clientX, y: e.clientY };

          if (dropZoneElement) {
            const dropZoneRect = dropZoneElement.getBoundingClientRect();
            const isInsideDropZone =
              e.clientX >= dropZoneRect.left &&
              e.clientX <= dropZoneRect.right &&
              e.clientY >= dropZoneRect.top &&
              e.clientY <= dropZoneRect.bottom;

            if (isInsideDropZone) {
              dragging.isOutsideDropZone = false;
              updateDropPosition(e);
            } else {
              dragging.isOutsideDropZone = true;
              dragging.dropIndex = null;
            }
          }

          const calculatedPosition = calculateWidgetPosition(e.clientX, e.clientY, dragging.widgetRect, dragging.offsetX, dragging.offsetY);

          dragging.calculatedPosition = calculatedPosition;
        }
      },
      onDragEnd: async () => {
        if (dragging && dragging.source === 'freePosition') {
          const currentDragging = dragging;
          currentDragging.dropped = true;

          try {
            const { widgetId, dropIndex } = currentDragging;
            const widget = $query.widgets.find((w) => w.id === widgetId);

            if (widget) {
              if (!currentDragging.isOutsideDropZone && dropIndex !== null) {
                await widgetContext.moveWidgetInGroup?.(widgetId, dropIndex);
              } else if (currentDragging.calculatedPosition) {
                await widgetContext.moveWidgetToFreePosition?.(widgetId, currentDragging.calculatedPosition);
              }
            }
          } finally {
            if (dragging === currentDragging) {
              dragging = null;
            }
          }
        }
      },
      onDragCancel: () => {
        dragging = null;
      },
    });

    return () => {
      dndHandler.destroy();
    };
  });

  $effect.pre(() => {
    void widgetsInGroup;
    if (!widgetListElement) return;

    animateFlip('[data-widget-flip-animation-id]', 'widgetFlipAnimationId', widgetListElement);
  });

  $effect(() => {
    if (!scrollContainerElement) return;

    scrollContainerElement.scrollTop = scrollContainerElement.scrollHeight;
  });

  $effect(() => {
    if (isHidden) {
      Tip.show('widget.hide', '`Alt` 키를 눌러 위젯을 잠시 투명하게 만들 수 있어요.');
    }
  });
</script>

<svelte:window
  onblur={() => {
    altPressed = false;
  }}
  onkeydown={(e) => {
    if (e.altKey) {
      altPressed = true;
    }
  }}
  onkeyup={(e) => {
    if (!e.altKey) {
      altPressed = false;
    }
  }}
/>

{#if dragging && !dragging.dropped}
  {@const tooltipMessage =
    dragging.source !== 'freePosition' && dragging.isOutsideDropZone
      ? '여기에 배치'
      : dragging.source === 'group' || dragging.isOutsideDropZone
        ? null
        : '그룹에 넣기'}
  {#if tooltipMessage}
    <div
      style:left="{dragging.cursorPosition.x}px"
      style:top="{dragging.cursorPosition.y}px"
      class={css({
        position: 'fixed',
        width: '1px',
        height: '1px',
        pointerEvents: 'none',
      })}
      use:tooltip={{ message: tooltipMessage, force: true, delay: 0, placement: 'top' }}
    ></div>
  {/if}
{/if}

<div
  style:transform={transformRight}
  class={cx(
    'group',
    css({
      position: 'fixed',
      bottom: '0',
      right: '24px',
      display: 'flex',
      flexDirection: 'column',
      justifyContent: 'flex-end',
      height: '[100dvh]',
      zIndex: 'widget',
      transition: '[transform 0.3s cubic-bezier(0.4, 0, 0.2, 1), opacity 0.2s ease-in-out]',
      pointerEvents: altPressed ? 'none!' : 'none',
      opacity: altPressed ? '15' : '100',
      '& *': {
        pointerEvents: transitioning || altPressed ? 'none!' : 'auto',
      },
    }),
  )}
  ontransitionend={(e) => {
    if (e.target === e.currentTarget) {
      transitioning = false;
    }
  }}
  ontransitionstart={(e) => {
    if (e.target === e.currentTarget) {
      transitioning = true;
    }
  }}
>
  <div
    bind:this={scrollContainerElement}
    class={flex({
      position: 'relative',
      flexDirection: 'column',
      backgroundColor: 'transparent',
      borderRadius: '12px',
      overflowY: 'auto',
      paddingBottom: '24px',
      scrollbarWidth: 'none',
      paddingTop: '8px',
      pointerEvents: 'auto',
    })}
    role="region"
  >
    {#if dragging && !dragging.dropped}
      <div
        bind:this={dropZoneElement}
        class={css({
          position: 'absolute',
          width: 'full',
          inset: '0',
          borderRadius: '12px',
          backgroundColor: 'black/15',
          pointerEvents: 'auto',
          zIndex: '[-1]',
        })}
      ></div>
    {/if}
    <div
      class={flex({
        position: 'relative',
        justifyContent: 'center',
        opacity: editMode ? '100' : '0',
        transitionProperty: '[opacity]',
        transitionDuration: '200ms',
        zIndex: '10',
        _groupHover: { opacity: '100' },
      })}
    >
      <Button
        style={css.raw({
          paddingX: '12px',
          paddingY: '4px',
          fontSize: '13px',
          borderRadius: 'full',
        })}
        onclick={() => {
          editMode = !editMode;
        }}
        size="sm"
        variant={editMode ? 'primary' : 'secondary'}
      >
        {editMode ? '완료' : '위젯 편집'}
      </Button>
      <div class={css({ position: 'absolute', right: '8px' })}>
        <div use:tooltip={{ message: '위젯 숨기기' }}>
          <Button
            style={css.raw({
              padding: '4px',
              borderRadius: 'full',
            })}
            onclick={() => {
              app.preference.current.widgetHidden = true;

              mixpanel.track('toggle_widget_visibility', {
                mode: 'hide',
              });

              editMode = false;
            }}
            size="sm"
            variant="secondary"
          >
            <Icon icon={XIcon} size={16} />
          </Button>
        </div>
      </div>
    </div>
    <div
      bind:this={widgetListElement}
      class={flex({
        flexDirection: 'column',
        justifyContent: 'flex-end',
        width: '[15dvw]',
        minWidth: '240px',
        maxWidth: '340px',
        margin: '8px',
        gap: '4px',
        position: 'relative',
      })}
      data-widget-group
    >
      {#if widgetsInGroup.length === 0 && (!dragging || dragging.dropped)}
        <div class={center({ flexDirection: 'column', gap: '12px', paddingY: '32px', paddingX: '16px' })}>
          <div class={center({ size: '48px', borderRadius: '12px', backgroundColor: 'surface.muted', color: 'text.faint' })}>
            <Icon icon={LayoutDashboardIcon} size={20} />
          </div>
          <p class={css({ fontSize: '13px', color: 'text.faint', textAlign: 'center', lineHeight: '[1.6]' })}>
            위젯 편집을 눌러
            <br />
            원하는 위젯을 추가해보세요
          </p>
        </div>
      {:else}
        <!-- NOTE: id와 index를 조합한 키를 쓰지 않으면 맨 아래에서 드래그할 때 dnd가 버벅거리는 경우 있음 -->
        {#each widgetsInGroup as item, index (`${item.id}-${index}`)}
          {#if item.type === 'droparea'}
            <div style:height={`${item.height}px`}></div>
          {:else if item.type === 'preview'}
            {@const WidgetComponent = WIDGET_COMPONENTS[item.widgetType]}
            <div class={css({ position: 'relative', opacity: '50' })} data-widget-flip-animation-id={item.widgetType}>
              <WidgetComponent data={item.widgetData} widgetId="drop-preview" />
            </div>
          {:else}
            {@const WidgetComponent = WIDGET_COMPONENTS[item.name as WidgetType]}
            <div
              class={cx('group', css({ position: 'relative' }))}
              data-widget-flip-animation-id={item.name}
              data-widget-id={item.id}
              role="listitem"
            >
              {#if editMode || item.name === 'onboarding'}
                <button
                  class={center({
                    position: 'absolute',
                    top: '0',
                    left: '0',
                    size: '24px',
                    borderRadius: 'full',
                    backgroundColor: 'surface.default',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    color: 'text.subtle',
                    transitionProperty: '[opacity]',
                    transitionDuration: '200ms',
                    transform: 'translate(-8px, -8px)',
                    _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
                    zIndex: '10',
                    cursor: 'pointer',
                  })}
                  onclick={(e) => {
                    e.preventDefault();
                    e.stopPropagation();
                    widgetContext.deleteWidget?.(item.id, 'button');
                  }}
                  onpointerdown={(e) => {
                    e.stopPropagation();
                  }}
                  type="button"
                >
                  <Icon icon={MinusIcon} size={14} />
                </button>
              {/if}
              <WidgetComponent data={item.data} widgetId={item.id} />
            </div>
          {/if}
        {/each}
      {/if}
    </div>
  </div>
</div>

<WidgetPalette
  $post={_post}
  addedWidgets={[
    ...widgetsInGroup.filter((w) => w.type === 'real').map((w) => w.name as WidgetType),
    ...freePositionWidgets.map((w) => w.name as WidgetType),
  ]}
  {editor}
  onDragCancel={() => {
    dragging = null;
  }}
  onDragEnd={async () => {
    if (dragging && dragging.source === 'palette') {
      const currentDragging = dragging;
      currentDragging.dropped = true;

      try {
        if (!currentDragging.isOutsideDropZone) {
          await widgetContext.createWidget?.(currentDragging.widgetType, 'drag', currentDragging.dropIndex ?? undefined);
        } else if (currentDragging.calculatedPosition) {
          await createWidgetMutation({
            name: currentDragging.widgetType,
            data: {
              position: currentDragging.calculatedPosition,
            },
          });

          mixpanel.track('create_widget', {
            widgetType: currentDragging.widgetType,
            via: 'drag_free',
          });
        }
        await cache.invalidate({ __typename: 'Query', field: 'widgets' });
      } finally {
        if (dragging === currentDragging) {
          dragging = null;
        }
      }
    }
  }}
  onDragMove={(e) => {
    updateDropPosition(e);

    if (dragging && dragging.source === 'palette') {
      const { widgetRect, offsetX, offsetY } = dragging;
      dragging.cursorPosition = { x: e.clientX, y: e.clientY };

      dragging.calculatedPosition = calculateWidgetPosition(e.clientX, e.clientY, widgetRect, offsetX, offsetY);
    }
  }}
  onDragStart={(e, widgetType, target) => {
    const rect = target.getBoundingClientRect();

    dragging = {
      dropIndex: null,
      isOutsideDropZone: true,
      cursorPosition: { x: e.clientX, y: e.clientY },
      source: 'palette',
      widgetType,
      widgetData: {},
      widgetRect: { width: rect.width, height: rect.height },
      offsetX: e.clientX - rect.left,
      offsetY: e.clientY - rect.top,
    };
  }}
  bind:open={editMode}
/>

<button
  class={center({
    position: 'fixed',
    bottom: '4px',
    right: '4px',
    size: '36px',
    borderRadius: '8px',
    zIndex: 'widget',
    pointerEvents: 'auto',
    cursor: 'pointer',
    borderWidth: '0',
    color: isHidden ? 'text.faint' : 'text.default',
    _hover: { color: 'text.default', backgroundColor: 'surface.muted' },
    transition: '[background-color 0.2s, color 0.2s]',
  })}
  aria-label={isHidden ? '위젯 보기' : '위젯 숨기기'}
  onclick={() => {
    app.preference.current.widgetHidden = !isHidden;
    mixpanel.track('toggle_widget_visibility', {
      mode: isHidden ? 'show' : 'hide',
    });
    if (!isHidden) {
      editMode = false;
    }
  }}
  type="button"
  use:tooltip={{ message: isHidden ? '위젯 보기' : '위젯 숨기기' }}
>
  <Icon icon={ShapesIcon} size={20} />
</button>

<div bind:this={freePositionListElement}>
  {#each freePositionWidgets as widget (widget.id)}
    {@const WidgetComponent = WIDGET_COMPONENTS[widget.name as WidgetType]}
    {@const position = widget.data.position as { top?: string; left?: string; bottom?: string; right?: string } | undefined}
    {@const isDragging = dragging?.source === 'freePosition' && dragging.widgetId === widget.id}
    {@const isDropped = isDragging && dragging?.dropped}
    {@const droppedPosition = isDragging ? dragging?.calculatedPosition : undefined}
    {@const toUsePos = isDropped && droppedPosition ? droppedPosition : position}
    {#if position && (!isDragging || isDropped)}
      <div
        style:top={toUsePos?.top}
        style:left={toUsePos?.left}
        style:bottom={toUsePos?.bottom}
        style:right={toUsePos?.right}
        class={cx(
          'group',
          css({
            position: 'fixed',
            width: '[15dvw]',
            minWidth: '240px',
            maxWidth: '340px',
            zIndex: 'widget',
            opacity: altPressed ? '15' : '100',
            transition: '[opacity 0.2s ease-in-out]',
            pointerEvents: altPressed ? 'none!' : 'auto',
          }),
        )}
        data-free-positioned
        data-widget-id={widget.id}
        role="listitem"
      >
        {#if editMode || widget.name === 'onboarding'}
          <button
            class={center({
              position: 'absolute',
              top: '0',
              left: '0',
              size: '24px',
              borderRadius: 'full',
              backgroundColor: 'surface.default',
              borderWidth: '1px',
              borderColor: 'border.default',
              color: 'text.subtle',
              transitionProperty: '[opacity]',
              transitionDuration: '200ms',
              transform: 'translate(-8px, -8px)',
              _hover: { backgroundColor: 'surface.subtle', color: 'text.default' },
              zIndex: '10',
              cursor: 'pointer',
            })}
            onclick={(e) => {
              e.preventDefault();
              e.stopPropagation();
              widgetContext.deleteWidget?.(widget.id, 'button');
            }}
            onpointerdown={(e) => {
              e.stopPropagation();
            }}
            type="button"
          >
            <Icon icon={MinusIcon} size={14} />
          </button>
        {/if}
        <WidgetComponent data={widget.data} widgetId={widget.id} />
      </div>
    {/if}
  {/each}
</div>

{#if dragging?.source === 'freePosition' && dragging.calculatedPosition && !dragging.dropped}
  {@const widget = $query.widgets.find((w) => dragging?.source === 'freePosition' && w.id === dragging.widgetId)}
  {#if widget}
    {@const WidgetComponent = WIDGET_COMPONENTS[widget.name as WidgetType]}
    <div
      style:top={dragging.calculatedPosition.top}
      style:left={dragging.calculatedPosition.left}
      style:bottom={dragging.calculatedPosition.bottom}
      style:right={dragging.calculatedPosition.right}
      class={cx(
        'group',
        css({
          position: 'fixed',
          width: '[15dvw]',
          minWidth: '240px',
          maxWidth: '340px',
          zIndex: 'widget',
          opacity: altPressed ? '15' : dragging.isOutsideDropZone ? '100' : '30',
          transition: '[opacity 0.2s ease-in-out]',
          pointerEvents: 'none',
        }),
      )}
    >
      <WidgetComponent data={widget.data} widgetId={widget.id} />
    </div>
  {/if}
{/if}
