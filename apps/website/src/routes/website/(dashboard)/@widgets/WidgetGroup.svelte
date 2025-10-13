<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Tip } from '@typie/ui/notification';
  import { animateFlip, createDndHandler, handleDragScroll } from '@typie/ui/utils';
  import { untrack } from 'svelte';
  import ChevronsLeftIcon from '~icons/lucide/chevrons-left';
  import ChevronsRightIcon from '~icons/lucide/chevrons-right';
  import LayoutDashboardIcon from '~icons/lucide/layout-dashboard';
  import MinusIcon from '~icons/lucide/minus';
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
  let hideTimeout = $state<NodeJS.Timeout | null>(null);
  let showTimeout = $state<NodeJS.Timeout | null>(null);
  let hovered = $state(false);
  let transitioning = $state(false);
  let altPressed = $state(false);

  type WidgetGroupState = 'hidden' | 'peeking' | 'visible';
  let widgetGroupState = $state<WidgetGroupState>('hidden');

  const transformRight = $derived.by(() => {
    if (!isHidden || editMode) {
      return 'translateX(0)';
    }

    switch (widgetGroupState) {
      case 'hidden': {
        return 'translateX(calc(100% + 24px))';
      }
      case 'peeking': {
        return 'translateX(calc(100% + 24px))';
      }
      case 'visible': {
        return 'translateX(0)';
      }
      default: {
        return 'translateX(calc(100% + 24px))';
      }
    }
  });

  $effect(() => {
    if (!isHidden) return;

    if (!hovered) {
      untrack(() => {
        if (hideTimeout) {
          clearTimeout(hideTimeout);
        }

        hideTimeout = setTimeout(() => {
          widgetGroupState = 'hidden';
          hideTimeout = null;
        }, 300);
      });
    }

    return () => {
      if (showTimeout) {
        clearTimeout(showTimeout);
        showTimeout = null;
      }

      if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
      }
    };
  });

  const handleMouseEnter = () => {
    if (transitioning) return;

    hovered = true;

    if (isHidden) {
      if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
      }

      if (showTimeout) {
        clearTimeout(showTimeout);
        showTimeout = null;
      }

      widgetGroupState = 'visible';
    }
  };

  const handleMouseLeave = () => {
    hovered = false;

    if (!isHidden) return;

    if (showTimeout) {
      clearTimeout(showTimeout);
      showTimeout = null;
    }

    if (hideTimeout) {
      clearTimeout(hideTimeout);
    }

    hideTimeout = setTimeout(() => {
      widgetGroupState = 'hidden';
      hideTimeout = null;
    }, 300);
  };

  const handleTriggerMouseEnter = () => {
    hovered = true;

    if (isHidden) {
      if (hideTimeout) {
        clearTimeout(hideTimeout);
        hideTimeout = null;
      }

      if (!showTimeout) {
        showTimeout = setTimeout(() => {
          widgetGroupState = 'visible';
          showTimeout = null;
        }, 150);
      }
    }
  };

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

  const updateDropPosition = async (e: PointerEvent) => {
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

  widgetContext.createWidget = async (type: WidgetType, index?: number) => {
    const widgets = widgetContext.state.widgets;
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

    cache.invalidate({ __typename: 'Query', field: 'widgets' });
  };

  widgetContext.deleteWidget = async (id: string) => {
    await deleteWidgetMutation({ widgetId: id });
    cache.invalidate({ __typename: 'Query', field: 'widgets' });
  };

  widgetContext.updateWidget = async (widgetId: string, data: Record<string, unknown>) => {
    await updateWidgetMutation({ widgetId, data });
    cache.invalidate({ __typename: 'Query', field: 'widgets' });
  };

  let localWidgetOrder = $state<string[]>([]);

  widgetContext.moveWidget = async (widgetId: string, targetIndex: number) => {
    const widgets = widgetContext.state.widgets;
    const currentIndex = widgets.findIndex((w) => w.id === widgetId);
    if (currentIndex === -1) return;

    const newOrder = widgets.map((w) => w.id);
    const [movedId] = newOrder.splice(currentIndex, 1);
    newOrder.splice(targetIndex, 0, movedId);
    localWidgetOrder = newOrder;

    const localWidgets = [...widgets]
      .toSorted((a, b) => {
        const indexA = localWidgetOrder.indexOf(a.id);
        const indexB = localWidgetOrder.indexOf(b.id);
        if (indexA === -1) return 1;
        if (indexB === -1) return -1;
        return indexA - indexB;
      })
      .filter((w) => dragging?.source === 'group' && dragging.widgetId !== w.id);

    let lowerOrder: string | undefined;
    let upperOrder: string | undefined;

    if (targetIndex === 0) {
      lowerOrder = undefined;
      upperOrder = localWidgets[1]?.order;
    } else if (targetIndex >= localWidgets.length - 1) {
      lowerOrder = localWidgets.at(-2)?.order;
      upperOrder = undefined;
    } else {
      lowerOrder = localWidgets[targetIndex - 1]?.order;
      upperOrder = localWidgets[targetIndex + 1]?.order;
    }

    try {
      await moveWidgetMutation({
        widgetId,
        lowerOrder,
        upperOrder,
      });
      cache.invalidate({ __typename: 'Query', field: 'widgets' });
    } catch (err) {
      localWidgetOrder = [];
      throw err;
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

  type WidgetItem = RealWidget | PreviewWidget;

  const localWidgets = $derived.by((): WidgetItem[] => {
    const widgets = widgetContext.state.widgets.filter((w) => !(dragging?.source === 'group' && dragging.widgetId === w.id));
    const sorted =
      localWidgetOrder.length === 0
        ? widgets.toSorted((a, b) => a.order.localeCompare(b.order))
        : [...widgets].toSorted((a, b) => {
            const indexA = localWidgetOrder.indexOf(a.id);
            const indexB = localWidgetOrder.indexOf(b.id);
            if (indexA === -1) return 1;
            if (indexB === -1) return -1;
            return indexA - indexB;
          });

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

    return result;
  });

  $effect(() => {
    if ($query?.widgets) {
      widgetContext.state.widgets = $query.widgets;
    }
  });

  $effect(() => {
    widgetContext.env.editMode = editMode;
    widgetContext.env.editor = editor;
    widgetContext.env.$post = _post;
  });

  let prevWidgetIds = $state<string[]>([]);
  $effect(() => {
    const widgetIds = $query?.widgets?.map((w) => w.id) ?? [];
    const widgetIdsStr = widgetIds.join(',');
    const prevWidgetIdsStr = prevWidgetIds.join(',');

    if (widgetIdsStr !== prevWidgetIdsStr) {
      prevWidgetIds = widgetIds;
      localWidgetOrder = widgetIds;
    }
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

        const widget = widgetContext.state.widgets.find((w) => w.id === widgetId);
        if (!widget) return;

        dragging = {
          dropIndex: widgetContext.state.widgets.findIndex((w) => w.id === widgetId),
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
          } else {
            await widgetContext.deleteWidget?.(dragging.widgetId);
          }
        }

        dragging = null;
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
    if (widgetGroupState === 'hidden' && isHidden) {
      Tip.show('widget.hide', '`Alt` 키를 눌러 위젯을 잠시 투명하게 만들 수 있어요.');
      Tip.show('widget.show', '커서를 화면 오른쪽 아래로 이동해 위젯을 다시 띄울 수 있어요.');
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
      backgroundColor: widgetGroupState === 'peeking' ? 'surface.dark/20' : 'transparent',
      borderWidth: widgetGroupState === 'peeking' ? '1px' : '0',
      borderColor: 'border.default',
      borderRadius: '12px',
      transition: '[background-color 0.2s ease-in-out]',
      overflowY: 'auto',
      paddingBottom: '24px',
      scrollbarWidth: 'none',
      paddingTop: '8px',
      pointerEvents: 'auto',
    })}
    onmouseenter={handleMouseEnter}
    onmouseleave={handleMouseLeave}
    role="region"
  >
    <div
      class={flex({
        position: 'relative',
        justifyContent: 'center',
        opacity: editMode || widgetGroupState === 'peeking' ? '100' : '0',
        transitionProperty: '[opacity]',
        transitionDuration: '200ms',
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
        <div use:tooltip={{ message: isHidden ? '위젯 고정' : '위젯 자동 숨김' }}>
          <Button
            style={css.raw({
              padding: '4px',
              borderRadius: 'full',
            })}
            onclick={() => {
              app.preference.current.widgetHidden = !app.preference.current.widgetHidden;
              if (app.preference.current.widgetHidden) {
                editMode = false;
                widgetGroupState = 'visible';
                setTimeout(() => {
                  if (!hovered) {
                    widgetGroupState = 'hidden';
                  }
                }, 300);
              }
            }}
            size="sm"
            variant="secondary"
          >
            <Icon icon={isHidden ? ChevronsLeftIcon : ChevronsRightIcon} size={16} />
          </Button>
        </div>
      </div>
    </div>
    <div
      class={flex({
        position: 'relative',
        flexDirection: 'column',
        gap: '8px',
        padding: '8px',
      })}
      data-widget-group
    >
      <div bind:this={widgetListElement} class={flex({ flexDirection: 'column', gap: '8px', position: 'relative' })}>
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
                      widgetContext.deleteWidget?.(item.id);
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
      await widgetContext.createWidget?.(dragging.widgetType, dragging.dropIndex ?? undefined);
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
  <div
    class={css({
      position: 'fixed',
      bottom: '0',
      right: '0',
      width: '40px',
      height: '40px',
      zIndex: 'widget',
      pointerEvents: 'auto',
    })}
    aria-label="위젯 표시 영역"
    onmouseenter={handleTriggerMouseEnter}
    onmouseleave={handleMouseLeave}
    role="region"
  ></div>
{/if}
