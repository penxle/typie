<script lang="ts">
  import { createFragment, createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { portal } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { elementScrollViewport, handleDragScroll } from '@typie/ui/utils';
  import mixpanel from 'mixpanel-browser';
  import { tick } from 'svelte';
  import { on } from 'svelte/events';
  import { SvelteMap } from 'svelte/reactivity';
  import { fade } from 'svelte/transition';
  import { DocumentType } from '#/enums';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import LayoutTemplateIcon from '~icons/lucide/layout-template';
  import { cache } from '$lib/graphql';
  import { graphql } from '$mearie';
  import { getPaneGroup } from '../[slug]/@pane/context.svelte';
  import SelectedEntitiesBar from './@selection/SelectedEntitiesBar.svelte';
  import Entity from './Entity.svelte';
  import { setupTreeContext } from './state.svelte';
  import { getNextElement, getPreviousElement, maxDepth } from './utils';
  import type { MouseEventHandler, PointerEventHandler } from 'svelte/elements';
  import type { DashboardLayout_EntityTree_site$key } from '$mearie';

  type EntityNode = {
    id: string;
    node: {
      __typename: 'Document' | 'Folder';
    };
    children?: EntityNode[];
    ' $_DashboardLayout_EntityTree_Entity_entity'?: unknown;
  };

  type Props = {
    site$key: DashboardLayout_EntityTree_site$key;
  };

  let { site$key }: Props = $props();

  const site = createFragment(
    graphql(`
      fragment DashboardLayout_EntityTree_site on Site {
        id

        entities {
          id

          node {
            __typename
          }

          ...DashboardLayout_EntityTree_Entity_entity
        }
      }
    `),
    () => site$key,
  );

  const [moveEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_EntityTree_MoveEntities_Mutation($input: MoveEntitiesInput!) {
        moveEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

              children {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }
          }

          children {
            id

            node {
              __typename
            }

            ...DashboardLayout_EntityTree_Entity_entity
          }

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
          }
        }
      }
    `),
  );

  const [deleteEntities] = createMutation(
    graphql(`
      mutation DashboardLayout_EntityTree_DeleteEntities_Mutation($input: DeleteEntitiesInput!) {
        deleteEntities(input: $input) {
          id

          site {
            id
            ...DashboardLayout_EntityTree_site
            ...DashboardLayout_TrashModal_site
          }

          container {
            ... on Site {
              id

              entities {
                id

                node {
                  __typename
                }

                ...DashboardLayout_EntityTree_Entity_entity
              }
            }

            ... on Entity {
              id

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

  type PendingTouchDrag = {
    event: PointerEvent;
    element: HTMLElement;
    startX: number;
    startY: number;
    lastY: number;
  };

  const TOUCH_LONG_PRESS_MS = 350;
  const TOUCH_MOVE_THRESHOLD = 10;

  let tree = $state<HTMLElement>();
  let dragging = $state<Dragging | null>(null);
  let pendingTouchDrag = $state<PendingTouchDrag | null>(null);
  let pointerType = $state<PointerEvent['pointerType']>('mouse');
  let dragTimeout = $state<NodeJS.Timeout | null>(null);
  let folderHoverTimeout = $state<NodeJS.Timeout | null>(null);
  let hoveredFolderId = $state<string | null>(null);

  let lastPointerX = $state<number>(0);
  let lastPointerY = $state<number>(0);

  const treeState = setupTreeContext();
  const paneGroup = getPaneGroup();

  $effect(() => {
    treeState.element = tree;
  });

  $effect(() => {
    if (site.data) {
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

      treeState.entities = collect(site.data.entities as unknown as EntityNode[]);
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

    // app.state.current는 URL slug 기반으로 +page.svelte에서 관리
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

  const clearDragTimeout = () => {
    if (dragTimeout) {
      clearTimeout(dragTimeout);
      dragTimeout = null;
    }
  };

  const clearPendingTouchDrag = () => {
    clearDragTimeout();
    pendingTouchDrag = null;
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

    if (pointerType === 'touch') {
      clearPendingTouchDrag();

      pendingTouchDrag = {
        event: e,
        element,
        startX: e.clientX,
        startY: e.clientY,
        lastY: e.clientY,
      };
      dragTimeout = setTimeout(() => {
        if (!pendingTouchDrag || pendingTouchDrag.event.pointerId !== e.pointerId) {
          return;
        }

        const rect = pendingTouchDrag.element.getBoundingClientRect();

        dragging = {
          eligible: true,
          event: pendingTouchDrag.event,
          element: pendingTouchDrag.element,
          indicator: {},
          ghost: {
            x: pendingTouchDrag.event.clientX,
            y: pendingTouchDrag.event.clientY,
            offsetX: pendingTouchDrag.event.clientX - rect.left,
            offsetY: pendingTouchDrag.event.clientY - rect.top,
          },
        };

        if (!pendingTouchDrag.element.hasPointerCapture(e.pointerId)) {
          pendingTouchDrag.element.setPointerCapture(e.pointerId);
        }

        lastPointerX = pendingTouchDrag.event.clientX;
        lastPointerY = pendingTouchDrag.event.clientY;
        pendingTouchDrag = null;
        dragTimeout = null;

        updateDropTarget(lastPointerX, lastPointerY);
      }, TOUCH_LONG_PRESS_MS);

      return;
    }

    clearPendingTouchDrag();

    if (pointerType === 'mouse' || pointerType === 'pen') {
      dragging = {
        eligible: false,
        event: e,
        element,
        indicator: {},
      };
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

    if (!targetElement && dragging.eligible && dragging.element.dataset.type === 'document') {
      const zone = paneGroup.hitTest(clientX, clientY);
      if (zone) {
        paneGroup.activeZone = zone;
        dragging.indicator = {};
        dragging.drop = { target: 'view' as const };
        clearFolderHoverTimeout();
        return;
      }
    }

    if (!targetElement) {
      if (paneGroup.activeZone) {
        paneGroup.cancelDrag();
      }

      dragging.indicator = {};
      dragging.drop = undefined;
      clearFolderHoverTimeout();
      return;
    }

    if (paneGroup.activeZone) {
      paneGroup.cancelDrag();
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

    return handleDragScroll(elementScrollViewport(scrollContainer), true, {
      onScroll: () => updateDropTarget(lastPointerX, lastPointerY),
    });
  });

  const handlePointerMove: PointerEventHandler<HTMLDivElement> = (e) => {
    if (!dragging || !tree) {
      if (pendingTouchDrag && pendingTouchDrag.event.pointerId === e.pointerId) {
        const movedDistance = Math.abs(pendingTouchDrag.startX - e.clientX) + Math.abs(pendingTouchDrag.startY - e.clientY);

        if (dragTimeout && movedDistance > TOUCH_MOVE_THRESHOLD) {
          clearDragTimeout();
        }

        if (!dragTimeout) {
          if (e.cancelable) {
            e.preventDefault();
          }

          const scrollContainer = tree?.parentElement;
          if (scrollContainer) {
            scrollContainer.scrollTop -= e.clientY - pendingTouchDrag.lastY;
          }
        }

        pendingTouchDrag.lastY = e.clientY;
      } else if (!dragging && !pendingTouchDrag) {
        clearDragTimeout();
      }

      return;
    }

    if (e.pointerId !== dragging.event.pointerId) {
      return;
    }

    lastPointerX = e.clientX;
    lastPointerY = e.clientY;

    if (dragging.eligible) {
      if (dragging.event.pointerType === 'touch' && e.cancelable) {
        e.preventDefault();
      }

      if (!dragging.element.hasPointerCapture(e.pointerId)) {
        dragging.element.setPointerCapture(e.pointerId);
      }

      dragging.ghost = {
        x: e.clientX,
        y: e.clientY,
        offsetX: dragging.ghost?.offsetX ?? 0,
        offsetY: dragging.ghost?.offsetY ?? 0,
      };

      updateDropTarget(e.clientX, e.clientY);
    } else if (Math.abs(dragging.event.clientX - e.clientX) + Math.abs(dragging.event.clientY - e.clientY) > TOUCH_MOVE_THRESHOLD) {
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

  const handlePointerUp: PointerEventHandler<HTMLDivElement> = async (e) => {
    if (!dragging) {
      if (pendingTouchDrag && pendingTouchDrag.event.pointerId !== e.pointerId) {
        return;
      }

      clearPendingTouchDrag();

      return;
    }

    if (e.pointerId !== dragging.event.pointerId) {
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
          await deleteEntities({ input: { entityIds: selectedIds } });

          cache.invalidate({ __typename: 'Site', id: site.data.id, $field: 'deletedEntities' });

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
          input: {
            entityIds: selectedIds,
            parentEntityId: parentId ?? null,
            lowerOrder,
            upperOrder,
          },
        });
        mixpanel.track('move_entities', { totalCount: selectedIds.length, parentEntityId: parentId ?? null, lowerOrder, upperOrder });
        return;
      } else if (target === 'view') {
        const entitySlug = dragging.element.dataset.slug;
        const entityType = dragging.element.dataset.type;
        if (entitySlug && entityType === 'document') {
          paneGroup.executeDrop({ slug: entitySlug, type: 'document' });
        } else {
          paneGroup.cancelDrag();
        }
        endDragging();
        return;
      }
    }

    endDragging(true);
  };

  const handlePointerCancel: PointerEventHandler<HTMLDivElement> = () => {
    if (dragging) {
      endDragging(true);
      return;
    }

    clearPendingTouchDrag();
  };

  const handleContextMenuCapture: MouseEventHandler<HTMLDivElement> = (e) => {
    if (pendingTouchDrag || (dragging && dragging.event.pointerType === 'touch')) {
      e.preventDefault();
      e.stopPropagation();
    }
  };

  const endDragging = (canceled = false) => {
    if (!dragging) {
      clearPendingTouchDrag();
      return;
    }

    clearPendingTouchDrag();

    clearFolderHoverTimeout();

    if (dragging.eligible && dragging.element.hasPointerCapture(dragging.event.pointerId)) {
      dragging.element.releasePointerCapture(dragging.event.pointerId);
    }

    if (canceled) {
      paneGroup.cancelDrag();
    }

    dragging = null;
  };

  const draggingEntityCount = $derived.by(() => {
    let count = {
      document: 0,
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
          count.document++;
        }
      });
    };

    collect(site.data.entities as unknown as EntityNode[]);

    return count;
  });

  const ghostEntityCount = $derived.by(() => {
    if (!dragging?.eligible) {
      return {
        document: 0,
        folder: 0,
      };
    }

    const entityId = dragging.element.dataset.id;
    const isMultipleDrag = entityId && treeState.selectedEntityIds.has(entityId) && treeState.selectedEntityIds.size > 1;
    if (isMultipleDrag) {
      return draggingEntityCount;
    }

    return {
      document: dragging.element.dataset.type === 'document' ? 1 : 0,
      folder: dragging.element.dataset.type === 'folder' ? 1 : 0,
    };
  });

  const ghostEntityName = $derived.by(() => {
    if (!dragging?.eligible) {
      return;
    }

    const entityId = dragging.element.dataset.id;
    const isMultipleDrag = entityId && treeState.selectedEntityIds.has(entityId) && treeState.selectedEntityIds.size > 1;
    if (isMultipleDrag) {
      return;
    }

    const name = dragging.element.dataset.name?.trim();
    return name ?? undefined;
  });

  const ghostEntityType = $derived.by(() => {
    return ghostEntityName ? dragging?.element.dataset.type : undefined;
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
  oncontextmenucapture={handleContextMenuCapture}
  onpointercancelcapture={handlePointerCancel}
  onpointerdowncapture={handlePointerDown}
  onpointermovecapture={handlePointerMove}
  onpointerupcapture={handlePointerUp}
  role="tree"
>
  {#each site.data.entities as entity (entity.id)}
    <Entity entity$key={entity} />
  {:else}
    <div class={center({ flexGrow: '1' })}>
      <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>아직 문서가 없어요</p>
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
        backgroundColor: { base: 'accent.info.default/30', _dark: 'accent.info.default/40' },
        pointerEvents: 'none',
        zIndex: 'sidebar',
      })}
      use:portal
      transition:fade|global={{ duration: 100 }}
    ></div>
  {/key}

  {#if dragging.ghost}
    <div
      style:left={`${dragging.ghost.x + 8}px`}
      style:top={`${dragging.ghost.y}px`}
      style:max-width={ghostEntityName ? `${dragging.element.offsetWidth}px` : undefined}
      class={flex({
        position: 'fixed',
        backgroundColor: 'accent.info.default',
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
      {#if ghostEntityName}
        <div class={flex({ alignItems: 'center', gap: '2px', minWidth: '0' })}>
          <Icon
            style={css.raw({ color: 'text.bright', flexShrink: '0' })}
            icon={ghostEntityType === 'folder'
              ? FolderIcon
              : dragging?.element.dataset.documentType === DocumentType.TEMPLATE
                ? LayoutTemplateIcon
                : FileIcon}
            size={14}
          />
          <span
            class={css({
              display: 'block',
              flexGrow: '1',
              minWidth: '0',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            })}
          >
            {ghostEntityName}
          </span>
        </div>
      {:else}
        {#if ghostEntityCount.folder > 0}
          <div class={center({ gap: '2px' })}>
            <Icon style={css.raw({ color: 'text.bright' })} icon={FolderIcon} size={14} />
            {ghostEntityCount.folder}
          </div>
        {/if}
        {#if ghostEntityCount.document > 0}
          <div class={center({ gap: '2px' })}>
            <Icon style={css.raw({ color: 'text.bright' })} icon={FileIcon} size={14} />
            {ghostEntityCount.document}
          </div>
        {/if}
      {/if}
    </div>
  {/if}
{/if}
