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
  import type { WidgetType } from './widget-context.svelte';

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

  type BaseDragging = {
    dropIndex: number | null;
    isOutsideDropZone: boolean;
    cursorPosition: { x: number; y: number };
    widgetType: WidgetType;
    widgetData: Record<string, unknown>;
  };

  type GroupDragging = BaseDragging & {
    source: 'group';
    widgetId: string;
  };

  type PaletteDragging = BaseDragging & {
    source: 'palette';
  };

  type DraggingState = GroupDragging | PaletteDragging;

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

    await cache.invalidate({ __typename: 'Query', field: 'widgets' });
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

      await cache.invalidate({ __typename: 'Query', field: 'widgets' });
    } catch (err) {
      optimisticDeletedWidgetIds = optimisticDeletedWidgetIds.filter((existingId) => existingId !== id);
      throw err;
    }
  };

  widgetContext.updateWidget = async (widgetId: string, data: Record<string, unknown>) => {
    await updateWidgetMutation({ widgetId, data });
    await cache.invalidate({ __typename: 'Query', field: 'widgets' });
  };

  widgetContext.moveWidget = async (widgetId: string, targetIndex: number) => {
    const widgets = $query.widgets;
    const currentIndex = widgets.findIndex((w) => w.id === widgetId);
    if (currentIndex === -1) return;

    const widget = widgets.find((w) => w.id === widgetId);

    const sortedWidgets = [...widgets]
      .toSorted((a, b) => a.order.localeCompare(b.order))
      .filter((w) => w.id !== widgetId && !optimisticDeletedWidgetIds.includes(w.id));

    let lowerOrder: string | undefined;
    let upperOrder: string | undefined;

    if (targetIndex === 0) {
      lowerOrder = undefined;
      upperOrder = sortedWidgets[0]?.order;
    } else if (targetIndex >= sortedWidgets.length) {
      lowerOrder = sortedWidgets.at(-1)?.order;
      upperOrder = undefined;
    } else {
      lowerOrder = sortedWidgets[targetIndex - 1]?.order;
      upperOrder = sortedWidgets[targetIndex]?.order;
    }

    await moveWidgetMutation({
      widgetId,
      lowerOrder,
      upperOrder,
    });

    mixpanel.track('move_widget', {
      widgetType: widget?.name,
    });

    await cache.invalidate({ __typename: 'Query', field: 'widgets' });
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

  type WidgetItem = RealWidget | PreviewWidget;

  let optimisticDeletedWidgetIds = $state<string[]>([]);
  let localWidgets = $state<WidgetItem[]>([]);
  let prevWidgetCount = 0;

  $effect.pre(() => {
    const newCount = $query.widgets.length;

    if (newCount < prevWidgetCount) {
      optimisticDeletedWidgetIds = optimisticDeletedWidgetIds.filter((id) => $query.widgets.find((w) => w.id === id));
    }

    // NOTE: 팔레트에서 드래그 중이고 위젯이 새로 추가되었다면 드래그 초기화. 해주지 않으면 순간적으로 drop preview와 새 위젯이 동시에 렌더링됨
    if (newCount > prevWidgetCount && dragging?.source === 'palette') {
      dragging = null;
    }

    prevWidgetCount = newCount;

    const widgets = $query.widgets.filter(
      (w) => !(dragging?.source === 'group' && dragging.widgetId === w.id) && !optimisticDeletedWidgetIds.includes(w.id),
    );
    const sorted = widgets.toSorted((a, b) => a.order.localeCompare(b.order));

    const result: WidgetItem[] = sorted.map((w) => ({ type: 'real' as const, ...w }));

    if (dragging?.widgetType && dragging.dropIndex !== null) {
      const previewWidget: PreviewWidget = {
        type: 'preview',
        id: 'drop-preview',
        widgetType: dragging.widgetType,
        widgetData: dragging.widgetData,
      };
      result.splice(dragging.dropIndex, 0, previewWidget);
    }

    localWidgets = result;
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

        dragging = {
          dropIndex: $query.widgets.findIndex((w) => w.id === widgetId),
          isOutsideDropZone: false,
          cursorPosition: { x: e.clientX, y: e.clientY },
          source: 'group',
          widgetId,
          widgetType: widget.name as WidgetType,
          widgetData: widget.data,
        };
      },
      onDragMove: (e) => {
        updateDropPosition(e);
      },
      onDragEnd: async (e) => {
        if (dragging && dragging.source === 'group' && dropZoneElement) {
          const rect = dropZoneElement.getBoundingClientRect();
          if (e.clientX >= rect.left && e.clientX <= rect.right && e.clientY >= rect.top && e.clientY <= rect.bottom) {
            if (dragging.dropIndex !== null) {
              await widgetContext.moveWidget?.(dragging.widgetId, dragging.dropIndex);
            }
            dragging = null;
          } else {
            const { widgetId } = dragging;
            dragging = null;
            await widgetContext.deleteWidget?.(widgetId, 'drag');
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

  $effect.pre(() => {
    void localWidgets;
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

{#if dragging?.isOutsideDropZone && dragging.source === 'group'}
  <div
    style:left="{dragging.cursorPosition.x}px"
    style:top="{dragging.cursorPosition.y}px"
    class={css({
      position: 'fixed',
      width: '1px',
      height: '1px',
      pointerEvents: 'none',
    })}
    use:tooltip={{ message: '제거', force: true, delay: 0, placement: 'top' }}
  ></div>
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
      width: '[15dvw]',
      minWidth: '256px',
      maxWidth: '356px',
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
  {#if dragging}
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
    bind:this={scrollContainerElement}
    class={flex({
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
      class={flex({ flexDirection: 'column', justifyContent: 'flex-end', padding: '8px', gap: '4px', position: 'relative' })}
      data-widget-group
    >
      {#if localWidgets.length === 0 && !dragging}
        <div
          class={flex({
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            gap: '12px',
            paddingY: '32px',
            paddingX: '16px',
          })}
        >
          <div
            class={flex({
              alignItems: 'center',
              justifyContent: 'center',
              width: '48px',
              height: '48px',
              borderRadius: '12px',
              backgroundColor: 'surface.muted',
              color: 'text.faint',
            })}
          >
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
        {#each localWidgets as item, index (`${item.id}-${index}`)}
          {#if item.type === 'preview'}
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
              {#if editMode}
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
  addedWidgets={localWidgets.filter((w) => w.type === 'real').map((w) => w.name as WidgetType)}
  {editor}
  onDragCancel={() => {
    dragging = null;
  }}
  onDragEnd={async () => {
    if (dragging && !dragging.isOutsideDropZone) {
      await widgetContext.createWidget?.(dragging.widgetType, 'drag', dragging.dropIndex ?? undefined);
    }
    dragging = null;
  }}
  onDragMove={(e) => {
    updateDropPosition(e);
  }}
  onDragStart={(e, widgetType) => {
    dragging = {
      dropIndex: null,
      isOutsideDropZone: true,
      cursorPosition: { x: e.clientX, y: e.clientY },
      source: 'palette',
      widgetType,
      widgetData: {},
    };
  }}
  bind:open={editMode}
/>

{#if isHidden}
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
      color: 'text.faint',
      _hover: { color: 'text.default', backgroundColor: 'surface.muted' },
    })}
    aria-label="위젯 보기"
    onclick={() => {
      app.preference.current.widgetHidden = false;
      mixpanel.track('toggle_widget_visibility', {
        mode: 'show',
      });
    }}
    type="button"
    use:tooltip={{ message: '위젯 보기' }}
  >
    <Icon icon={ShapesIcon} size={20} />
  </button>
{/if}
