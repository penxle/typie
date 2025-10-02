<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { Toast } from '@typie/ui/notification';
  import { handleDragScroll } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { on } from 'svelte/events';
  import { SvelteMap } from 'svelte/reactivity';
  import { fade } from 'svelte/transition';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import { fragment, graphql } from '$graphql';
  import { getDragDropContext } from '../[slug]/@split-view/drag-context.svelte';
  import SelectedEntitiesBar from './@selection/SelectedEntitiesBar.svelte';
  import Entity from './Entity.svelte';
  import { setupTreeContext } from './state.svelte';
  import { getNextElement, getPreviousElement, maxDepth } from './utils';
  import type { MouseEventHandler, PointerEventHandler } from 'svelte/elements';
  import type { DashboardLayout_EntityTree_site } from '$graphql';

  type EntityNode = {
    id: string;
    node: {
      __typename: 'Canvas' | 'Folder' | 'Post';
    };
    children?: EntityNode[];
    ' $$_DashboardLayout_EntityTree_Entity_entity'?: unknown;
  };

  type Props = {
    $site: DashboardLayout_EntityTree_site;
  };

  let { $site: _site }: Props = $props();

  const site = fragment(
    _site,
    graphql(`
      fragment DashboardLayout_EntityTree_site on Site {
        id

        entities {
          id
          node {
            __typename
          }
          ...DashboardLayout_EntityTree_Entity_entity

          children {
            id
            node {
              __typename
            }
            ...DashboardLayout_EntityTree_Entity_entity

            children {
              id
              node {
                __typename
              }
              ...DashboardLayout_EntityTree_Entity_entity

              children {
                id
                node {
                  __typename
                }
                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }
        }
      }
    `),
  );

  const moveEntities = graphql(`
    mutation DashboardLayout_EntityTree_MoveEntities_Mutation($input: MoveEntitiesInput!) {
      moveEntities(input: $input) {
        id

        ancestors {
          id

          node {
            __typename

            ... on Folder {
              id
              name
            }
          }
        }

        parent {
          id

          children {
            id
            slug

            node {
              __typename
            }
          }
        }
      }
    }
  `);

  const deleteEntities = graphql(`
    mutation DashboardLayout_EntityTree_DeleteEntities_Mutation($input: DeleteEntitiesInput!) {
      deleteEntities(input: $input) {
        id
        site {
          id
          ...DashboardLayout_EntityTree_site
          ...DashboardLayout_Trash_site
          ...DashboardLayout_PlanUsageWidget_site
        }
      }
    }
  `);

  type Indicator = {
    top: number;
    left: number;
    width: number;
    height: number;
    opacity: number;
    transform: string;
  };

  type TreeDrop = {
    target: 'tree';
    parentId?: string;
    lowerOrder?: string;
    upperOrder?: string;
  };

  type TrashDrop = {
    target: 'trash';
  };

  type ViewDrop = {
    target: 'view';
    viewId: string;
  };

  type Drop = TreeDrop | TrashDrop | ViewDrop;

  type Dragging = {
    eligible: boolean;
    event: PointerEvent;
    element: HTMLElement;
    indicator: Partial<Indicator>;
    drop?: Drop;
    ghost?: {
      x: number;
      y: number;
      offsetX: number;
      offsetY: number;
    };
  };

  let tree = $state<HTMLElement>();
  let dragging = $state<Dragging | null>(null);
  let pointerType = $state<PointerEvent['pointerType']>('mouse');
  let dragTimeout = $state<NodeJS.Timeout | null>(null);
  let folderHoverTimeout = $state<NodeJS.Timeout | null>(null);
  let hoveredFolderId = $state<string | null>(null);

  let lastPointerX = $state<number>(0);
  let lastPointerY = $state<number>(0);

  const app = getAppContext();
  const treeState = setupTreeContext();
  const dragDropContext = getDragDropContext();

  $effect(() => {
    treeState.element = tree;
  });

  $effect(() => {
    if ($site) {
      const entityMap = new SvelteMap<string, (typeof treeState.entities)[number]>();
      const collect = (children: EntityNode[], parentId?: string): typeof treeState.entities => {
        const entities = children.map((entity) => ({
          id: entity.id,
          type: entity.node.__typename,
          children: entity.children ? collect(entity.children, entity.id) : undefined,
          parentId,
        }));

        for (const entity of entities) {
          entityMap.set(entity.id, entity);
        }

        return entities;
      };

      treeState.entities = collect($site.entities);
      treeState.entityMap = entityMap;
    }
  });

  $effect(() => {
    if (!treeState.entityMap) return;

    const validEntityIds = new Set(treeState.entityMap.keys());

    const invalidSelectedIds = treeState.selectedEntityIds.difference(validEntityIds);
    for (const entityId of invalidSelectedIds) {
      treeState.selectedEntityIds.delete(entityId);
    }

    if (treeState.lastSelectedEntityId && !validEntityIds.has(treeState.lastSelectedEntityId)) {
      treeState.lastSelectedEntityId = undefined;
    }

    if (app.state.current && !validEntityIds.has(app.state.current)) {
      app.state.ancestors = [];
      app.state.current = undefined;
    }
  });

  // NOTE: 링크 관련 브라우저 기본 동작 방지
  const handleClick: MouseEventHandler<HTMLDivElement> = (e) => {
    const element = (e.target as HTMLElement).closest<HTMLElement>('[data-id]');

    if (!element) {
      return;
    }

    const entityId = element.dataset.id;

    if (entityId && (e.shiftKey || e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();
    }
  };

  const handlePointerDown: PointerEventHandler<HTMLDivElement> = (e) => {
    // NOTE: 우클릭 무시
    if (e.button === 2) {
      return;
    }

    const target = e.target as HTMLElement;

    const isMenuRelated = target.closest('[aria-pressed]') || target.closest('[role="menu"]') || target.closest('[role="menuitem"]');

    if (isMenuRelated) {
      return;
    }

    const element = target.closest<HTMLElement>('[data-id]');

    if (!element) {
      return;
    }

    pointerType = e.pointerType;

    if (pointerType === 'mouse') {
      dragging = {
        eligible: false,
        event: e,
        element,
        indicator: {},
      };
    } else {
      dragTimeout = setTimeout(() => {
        dragging = {
          eligible: false,
          event: e,
          element,
          indicator: {},
        };
      }, 50);
    }
  };

  const clearFolderHoverTimeout = () => {
    if (folderHoverTimeout) {
      clearTimeout(folderHoverTimeout);
    }
    folderHoverTimeout = null;
    hoveredFolderId = null;
  };

  const updateDropTarget = (clientX: number, clientY: number) => {
    if (!dragging || !tree) return;

    const trashElement = document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[data-type="trash"]');

    if (trashElement) {
      const rect = trashElement.getBoundingClientRect();
      dragging.indicator = {
        top: rect.top,
        left: rect.left,
        width: rect.width,
        height: rect.height,
        opacity: 0.5,
        transform: undefined,
      };
      dragging.drop = { target: 'trash' };
      clearFolderHoverTimeout();
      return;
    }

    const targetElement =
      document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[data-id]') ??
      document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[role="tree"]')?.querySelector('& > [data-id]:last-child');

    const splitViewElement = document.elementFromPoint(clientX, clientY)?.closest<HTMLElement>('[data-view-id]');

    if (!targetElement && splitViewElement && dragging.eligible) {
      if (dragDropContext && !dragDropContext.state.isDragging) {
        const entityType = dragging.element.dataset.type;
        const entitySlug = dragging.element.dataset.slug;

        if (entitySlug && entityType && ['post', 'canvas'].includes(entityType)) {
          dragDropContext.startDrag({
            slug: entitySlug,
            type: entityType as 'post' | 'canvas',
          });
        }
      }

      dragging.indicator = {};
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      dragging.drop = { target: 'view' as const, viewId: splitViewElement.dataset.viewId! };
      clearFolderHoverTimeout();
      return;
    }

    if (!targetElement) {
      clearFolderHoverTimeout();
      return;
    }

    if (dragDropContext.state.isDragging) {
      dragDropContext.cancelDrag();
    }

    const entityId = dragging.element.dataset.id;
    const isMultipleDrag = entityId && treeState.selectedEntityIds.has(entityId) && treeState.selectedEntityIds.size > 1;

    let isCycle = false;
    if (isMultipleDrag) {
      for (const selectedId of treeState.selectedEntityIds) {
        const selectedElement = tree?.querySelector(`[data-id="${selectedId}"]`);
        if (selectedElement?.contains(targetElement)) {
          isCycle = true;
          break;
        }
      }
    } else {
      isCycle = dragging.element.contains(targetElement);
    }
    if (isCycle) {
      dragging.indicator = {};
      dragging.drop = undefined;
      return;
    }

    const anchorElement = targetElement.querySelector<HTMLElement>(':scope > [data-anchor="true"]') ?? targetElement;
    if (!anchorElement) {
      return;
    }

    const targetRect = targetElement.getBoundingClientRect();
    const anchorRect = anchorElement.getBoundingClientRect();

    const relativeY = clientY - anchorRect.top;
    const thresholdY = 5;

    let offsetElement;
    let parentElement;

    if (targetElement.dataset.type === 'folder' && relativeY > thresholdY && relativeY < anchorRect.height - thresholdY) {
      dragging.indicator.top = targetRect.top;
      dragging.indicator.height = targetRect.height;
      dragging.indicator.opacity = 0.5;
      dragging.indicator.transform = undefined;

      parentElement = targetElement;

      dragging.drop = {
        target: 'tree',
        lowerOrder: [...targetElement.querySelectorAll<HTMLElement>('[data-id]')].at(-1)?.dataset.order,
      };

      const folderId = targetElement.dataset.id;
      const detailsElement = targetElement.closest('details');

      if (!folderId || !detailsElement) {
        clearFolderHoverTimeout();
        return;
      }

      if (hoveredFolderId !== folderId || !detailsElement.open) {
        if (folderHoverTimeout) {
          clearTimeout(folderHoverTimeout);
          folderHoverTimeout = null;
        }

        hoveredFolderId = folderId;

        if (!detailsElement.open) {
          folderHoverTimeout = setTimeout(async () => {
            if (hoveredFolderId === folderId && dragging?.eligible && detailsElement) {
              detailsElement.open = true;
              await tick();
              updateDropTarget(clientX, clientY);
            }
          }, 500);
        }
      }
    } else {
      clearFolderHoverTimeout();

      if (relativeY < anchorRect.height / 2) {
        offsetElement = getPreviousElement(tree, targetElement, '[data-id]');
        dragging.indicator.top = anchorRect.top;

        const thisElement = offsetElement ?? targetElement;
        parentElement = thisElement.closest<HTMLElement>(`[data-id]:not([data-id="${thisElement.dataset.id}"])`);

        dragging.drop = {
          target: 'tree',
          lowerOrder: offsetElement?.dataset.order,
          upperOrder: targetElement.dataset.order,
        };
      } else {
        if (targetElement.dataset.type === 'folder') {
          offsetElement = targetElement;
          dragging.indicator.top = targetRect.top + targetRect.height;

          parentElement = targetElement.closest<HTMLElement>(`[data-id]:not([data-id="${targetElement.dataset.id}"])`);

          dragging.drop = {
            target: 'tree',
            lowerOrder: targetElement.dataset.order,
          };
        } else {
          offsetElement = getNextElement(tree, targetElement, '[data-id]');
          dragging.indicator.top = anchorRect.top + anchorRect.height;

          const thisElement = offsetElement ?? targetElement;
          parentElement = thisElement.closest<HTMLElement>(`[data-id]:not([data-id="${thisElement.dataset.id}"])`);

          dragging.drop = {
            target: 'tree',
            lowerOrder: targetElement.dataset.order,
            upperOrder: offsetElement?.dataset.order,
          };
        }
      }

      dragging.indicator.height = 4;
      dragging.indicator.opacity = 1;
      dragging.indicator.transform = 'translateY(-50%)';
    }

    if (offsetElement) {
      const offsetRect = offsetElement.getBoundingClientRect();
      dragging.indicator.left = offsetRect.left;
      dragging.indicator.width = offsetRect.width;
    } else {
      dragging.indicator.left = anchorRect.left;
      dragging.indicator.width = anchorRect.width;
    }

    if (parentElement) {
      dragging.drop.target = 'tree';
      dragging.drop.parentId = parentElement.dataset.id;

      if (dragging.element.dataset.type === 'folder') {
        const newPathDepth = Number(parentElement.dataset.pathDepth ?? 0) + 1;
        const folderDepth = 1;
        const draggingDepth = Math.max(
          folderDepth,
          ...[...dragging.element.querySelectorAll<HTMLElement>('[data-type="folder"]')].map(
            (element) => Number(element.dataset.pathDepth ?? 0) - Number(dragging?.element.dataset.pathDepth ?? 0) + folderDepth,
          ),
        );

        if (newPathDepth + draggingDepth > maxDepth) {
          dragging.indicator = {};
          dragging.drop = undefined;
          return;
        }
      }
    }
  };

  $effect(() => {
    if (!dragging?.eligible || !tree) return;

    const scrollContainer = tree.parentElement;
    if (!scrollContainer) return;

    return handleDragScroll(scrollContainer, true, {
      onScroll: () => updateDropTarget(lastPointerX, lastPointerY),
    });
  });

  const handlePointerMove: PointerEventHandler<HTMLDivElement> = (e) => {
    if (!dragging || !tree) {
      if (dragTimeout) {
        clearTimeout(dragTimeout);
        dragTimeout = null;
      }

      return;
    }

    lastPointerX = e.clientX;
    lastPointerY = e.clientY;

    const isPostsPanelVisible = app.state.postsOpen || app.preference.current.postsExpanded === 'open';
    if (dragging.eligible && !isPostsPanelVisible) {
      endDragging();
      return;
    }

    if (dragging.eligible) {
      dragging.ghost = {
        x: e.clientX,
        y: e.clientY,
        offsetX: dragging.ghost?.offsetX ?? 0,
        offsetY: dragging.ghost?.offsetY ?? 0,
      };

      updateDropTarget(e.clientX, e.clientY);
    } else if (Math.abs(dragging.event.clientX - e.clientX) + Math.abs(dragging.event.clientY - e.clientY) > 10) {
      dragging.eligible = true;
      dragging.element.setPointerCapture(e.pointerId);

      const rect = dragging.element.getBoundingClientRect();
      dragging.ghost = {
        x: dragging.event.clientX,
        y: dragging.event.clientY,
        offsetX: dragging.event.clientX - rect.left,
        offsetY: dragging.event.clientY - rect.top,
      };
    }
  };

  const handlePointerUp: PointerEventHandler<HTMLDivElement> = async () => {
    if (!dragging) {
      if (dragTimeout) {
        clearTimeout(dragTimeout);
        dragTimeout = null;
      }

      return;
    }

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    const entityId = dragging.element.dataset.id!;

    if (dragging.eligible) {
      on(window, 'click', (e) => e.preventDefault(), { capture: true, once: true });
    }

    if (dragging.drop) {
      const { target } = dragging.drop;

      const isMultipleDrag = treeState.selectedEntityIds.size > 1 && treeState.selectedEntityIds.has(entityId);
      const selectedIds = isMultipleDrag ? [...treeState.selectedEntityIds] : [entityId];

      if (target === 'trash') {
        try {
          endDragging();
          await deleteEntities({ entityIds: selectedIds });

          mixpanel.track('delete_entities', { totalCount: selectedIds.length, via: 'drag_and_drop' });

          treeState.selectedEntityIds.clear();
          treeState.lastSelectedEntityId = undefined;

          Toast.success(`${selectedIds.length}개의 항목이 삭제되었어요`);
        } catch {
          Toast.error('삭제 중 오류가 발생했습니다');
        }
        return;
      } else if (target === 'tree') {
        const { parentId, lowerOrder, upperOrder } = dragging.drop;

        endDragging();
        await moveEntities({
          entityIds: selectedIds,
          parentEntityId: parentId ?? null,
          lowerOrder,
          upperOrder,
        });
        mixpanel.track('move_entities', { totalCount: selectedIds.length, parentEntityId: parentId ?? null, lowerOrder, upperOrder });
        return;
      } else if (target === 'view') {
        endDragging();
        // NOTE: DropZone 에서 처리
        return;
      }
    }

    endDragging(true);
  };

  const endDragging = (canceled = false) => {
    if (!dragging) {
      return;
    }

    if (dragTimeout) {
      clearTimeout(dragTimeout);
      dragTimeout = null;
    }

    clearFolderHoverTimeout();

    if (dragging.eligible && dragging.element.hasPointerCapture(dragging.event.pointerId)) {
      dragging.element.releasePointerCapture(dragging.event.pointerId);
    }

    if (dragDropContext.state.isDragging) {
      if (canceled) {
        dragDropContext.cancelDrag();
      } else {
        dragDropContext.drop();
      }
    }

    dragging = null;
  };

  const draggingEntityCount = $derived.by(() => {
    let count = {
      post: 0,
      canvas: 0,
      folder: 0,
    };

    const entityIds = new Set(treeState.selectedEntityIds);

    const collect = (entities: EntityNode[]) => {
      entities.forEach((entity) => {
        if (entity.node.__typename === 'Folder') {
          if (entityIds.has(entity.id)) {
            count.folder++;
          }

          if (entity.children) {
            collect(entity.children);
          }
        } else if (entityIds.has(entity.id)) {
          if (entity.node.__typename === 'Post') {
            count.post++;
          } else if (entity.node.__typename === 'Canvas') {
            count.canvas++;
          }
        }
      });
    };

    collect($site.entities as EntityNode[]);

    return count;
  });
</script>

<svelte:window
  oncontextmenu={(e) => {
    if (pointerType === 'mouse') {
      endDragging(true);
    } else {
      e.preventDefault();
      e.stopPropagation();
    }
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      endDragging(true);
    }
  }}
/>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_interactive_supports_focus -->
<div
  bind:this={tree}
  class={flex({
    flexDirection: 'column',
    minHeight: 'full',
    userSelect: 'none',
    touchAction: 'none',
  })}
  onclick={handleClick}
  onpointerdowncapture={handlePointerDown}
  onpointermovecapture={handlePointerMove}
  onpointerupcapture={handlePointerUp}
  role="tree"
>
  {#each $site.entities as entity (entity.id)}
    <Entity $entity={entity} />
  {:else}
    <div class={center({ flexGrow: '1' })}>
      <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>아직 포스트가 없어요</p>
    </div>
  {/each}

  {#if treeState.selectedEntityIds.size > 0 && !dragging?.eligible}
    <SelectedEntitiesBar />
  {/if}
</div>

{#if dragging?.eligible}
  {#key JSON.stringify(dragging.indicator)}
    <div
      style:top={`${dragging.indicator.top ?? -1}px`}
      style:left={`${dragging.indicator.left ?? -1}px`}
      style:width={`${dragging.indicator.width ?? 0}px`}
      style:height={`${dragging.indicator.height ?? 0}px`}
      style:opacity={dragging.indicator.opacity}
      style:transform={dragging.indicator.transform}
      class={css({
        position: 'fixed',
        borderRadius: '2px',
        backgroundColor: 'accent.brand.subtle',
        pointerEvents: 'none',
        zIndex: 'sidebar',
      })}
      use:portal
      transition:fade|global={{ duration: 100 }}
    ></div>
  {/key}

  {#if dragging.ghost}
    {@const entityId = dragging.element.dataset.id}
    {@const isMultipleDrag = entityId && treeState.selectedEntityIds.has(entityId) && treeState.selectedEntityIds.size > 1}
    {#if isMultipleDrag}
      <div
        style:left={`${dragging.ghost.x + 8}px`}
        style:top={`${dragging.ghost.y}px`}
        class={flex({
          position: 'fixed',
          backgroundColor: 'accent.brand.default',
          opacity: dragging.drop ? undefined : '[0.5]',
          gap: '8px',
          color: 'text.bright',
          alignItems: 'center',
          justifyContent: 'center',
          paddingX: '8px',
          paddingY: '4px',
          borderRadius: 'full',
          fontSize: '14px',
          fontWeight: 'bold',
          pointerEvents: 'none',
          zIndex: 'ghost',
        })}
        use:portal
      >
        {#if draggingEntityCount.folder > 0}
          <div class={center({ gap: '2px' })}>
            <Icon style={css.raw({ color: 'text.bright' })} icon={FolderIcon} size={14} />
            {draggingEntityCount.folder}
          </div>
        {/if}
        {#if draggingEntityCount.post > 0}
          <div class={center({ gap: '2px' })}>
            <Icon style={css.raw({ color: 'text.bright' })} icon={FileIcon} size={14} />
            {draggingEntityCount.post}
          </div>
        {/if}
        {#if draggingEntityCount.canvas > 0}
          <div class={center({ gap: '2px' })}>
            <Icon style={css.raw({ color: 'text.bright' })} icon={LineSquiggleIcon} size={14} />
            {draggingEntityCount.canvas}
          </div>
        {/if}
      </div>
    {:else}
      <div
        style:left={`${dragging.ghost.x - dragging.ghost.offsetX}px`}
        style:top={`${dragging.ghost.y - dragging.ghost.offsetY}px`}
        style:width={`${dragging.element.offsetWidth}px`}
        class={css({
          position: 'fixed',
          opacity: '[0.2]',
          pointerEvents: 'none',
          zIndex: 'ghost',
        })}
        use:portal
      >
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html dragging.element.outerHTML}
      </div>
    {/if}
  {/if}
{/if}
