<script lang="ts">
  import { cache } from '@typie/sark/internal';
  import { css, cx } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Button, Icon } from '@typie/ui/components';
  import { animateFlip, createDndHandler, handleDragScroll } from '@typie/ui/utils';
  import ChevronsDownIcon from '~icons/lucide/chevrons-down';
  import ShapesIcon from '~icons/lucide/shapes';
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

  let editMode = $state(false);
  let isHidden = $state(false);

  let dropZoneElement = $state<HTMLDivElement>();
  let widgetListElement = $state<HTMLDivElement>();
  let scrollContainerElement = $state<HTMLDivElement>();

  let dragging = $state<{
    dropIndex: number | null;
    isOutsideDropZone: boolean;
    cursorPosition: { x: number; y: number };
    source: 'group' | 'palette';
    widgetId: string;
    widgetType: WidgetType;
    widgetData: Record<string, unknown>;
  } | null>(null);

  const updateDropPosition = (e: PointerEvent) => {
    if (!dropZoneElement || !dragging) {
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

    const widgetElements = [...dropZoneElement.querySelectorAll('[data-widget-id]')] as HTMLElement[];
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
      .filter((w) => dragging?.widgetId !== w.id);

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

  const localWidgets = $derived.by(() => {
    const widgets = widgetContext.state.widgets.filter((w) => dragging?.widgetId !== w.id);
    if (localWidgetOrder.length === 0) {
      return widgets.toSorted((a, b) => a.order.localeCompare(b.order));
    }
    return [...widgets].toSorted((a, b) => {
      const indexA = localWidgetOrder.indexOf(a.id);
      const indexB = localWidgetOrder.indexOf(b.id);
      if (indexA === -1) return 1;
      if (indexB === -1) return -1;
      return indexA - indexB;
    });
  });

  $effect(() => {
    if ($query?.widgets) {
      widgetContext.state.widgets = $query.widgets;
    }
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

  const hasWidgets = $derived(localWidgets.length > 0);

  $effect(() => {
    if (!dropZoneElement || !widgetListElement) return;

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
          dropIndex: null,
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
        if (dragging && dropZoneElement) {
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
    void dragging?.dropIndex;
    void widgetContext.state.widgets;
    if (!dropZoneElement) return;

    animateFlip('[data-widget-id]', 'widgetId', dropZoneElement);
  });
</script>

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
  bind:this={scrollContainerElement}
  class={cx(
    'group',
    css({
      position: 'fixed',
      bottom: '0',
      right: '24px',
      display: 'flex',
      flexDirection: 'column',
      width: '[15dvw]',
      minWidth: '256px',
      maxWidth: '356px',
      maxHeight: '[calc(100dvh - 128px)]',
      paddingTop: '8px',
      zIndex: 'widget',
      overflowY: 'auto',
      scrollbarWidth: 'none',
      pointerEvents: 'none',
    }),
  )}
>
  {#if !isHidden}
    <div
      class={flex({
        position: 'relative',
        justifyContent: 'center',
        opacity: editMode ? '100' : '0',
        transitionProperty: '[opacity]',
        transitionDuration: '200ms',
        _groupHover: { opacity: '100' },
        pointerEvents: 'auto',
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
              isHidden = true;
              editMode = false;
            }}
            size="sm"
            variant="secondary"
          >
            <Icon icon={ChevronsDownIcon} size={16} />
          </Button>
        </div>
      </div>
    </div>
  {:else}
    <div
      class={css({
        position: 'fixed',
        bottom: '16px',
        right: '24px',
        zIndex: 'modal',
        pointerEvents: 'auto',
      })}
      use:tooltip={{ message: '위젯 보이기' }}
    >
      <Button
        style={css.raw({
          padding: '4px',
          borderRadius: 'full',
        })}
        onclick={() => {
          isHidden = false;
        }}
        size="sm"
        variant="secondary"
      >
        <Icon icon={ShapesIcon} size={16} />
      </Button>
    </div>
  {/if}

  {#if !isHidden}
    <div
      bind:this={dropZoneElement}
      class={flex({
        position: 'relative',
        flexDirection: 'column',
        gap: '8px',
        padding: '8px',
      })}
      data-widget-group
    >
      {#if dragging}
        <div
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

      <div bind:this={widgetListElement} class={flex({ flexDirection: 'column', gap: '8px', position: 'relative' })}>
        {#each localWidgets as widget, index (widget.id)}
          {@const WidgetComponent = WIDGET_COMPONENTS[widget.name as WidgetType]}
          {@const isDragging = dragging?.source === 'group' && dragging?.widgetId === widget.id}
          {#if dragging?.dropIndex === index && dragging?.widgetType}
            {@const DraggingWidgetComponent = WIDGET_COMPONENTS[dragging.widgetType]}
            <div style:opacity="0.5">
              <DraggingWidgetComponent $post={_post} data={dragging.widgetData} {editor} widgetId="drop-preview" />
            </div>
          {/if}
          {#if !isDragging}
            <div style:pointer-events="auto" data-widget-id={widget.id} role="listitem">
              <WidgetComponent $post={_post} data={widget.data} {editMode} {editor} widgetId={widget.id} />
            </div>
          {/if}
        {/each}
        {#if dragging?.dropIndex === localWidgets.length && dragging.widgetType}
          {@const DraggingWidgetComponent = WIDGET_COMPONENTS[dragging.widgetType]}
          <div style:opacity="0.5">
            <DraggingWidgetComponent $post={_post} data={dragging.widgetData} {editor} widgetId="drop-preview" />
          </div>
        {/if}
        {#if !hasWidgets && dragging?.dropIndex === null}
          <div
            class={css({
              width: 'full',
              height: '50px',
            })}
          ></div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<WidgetPalette
  $post={_post}
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
      widgetId: '',
      widgetType,
      widgetData: {},
    };
  }}
  bind:open={editMode}
/>
