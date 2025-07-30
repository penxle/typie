<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { on } from 'svelte/events';
  import { fade } from 'svelte/transition';
  import FileIcon from '~icons/lucide/file';
  import FolderIcon from '~icons/lucide/folder';
  import LineSquiggleIcon from '~icons/lucide/line-squiggle';
  import { fragment, graphql } from '$graphql';
  import { portal } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import Entity from './Entity.svelte';
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

        ...DashboardLayout_EntityTree_MultiEntitiesMenu_site

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

  type Indicator = {
    top: number;
    left: number;
    width: number;
    height: number;
    opacity: number;
    transform: string;
  };

  type Drop = {
    parentId?: string;
    lowerOrder?: string;
    upperOrder?: string;
  };

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

  const app = getAppContext();

  const getAllEntityIds = () => {
    const ids: string[] = [];
    const collectIds = (entities: EntityNode[]) => {
      entities.forEach((entity) => {
        ids.push(entity.id);

        if (entity.children && tree) {
          const folderElement = tree.querySelector(`[data-id="${entity.id}"]`) as HTMLDetailsElement;
          const isOpen = folderElement?.open ?? false;

          if (isOpen) {
            collectIds(entity.children as EntityNode[]);
          }
        }
      });
    };
    collectIds($site.entities as EntityNode[]);
    return ids;
  };

  const toggleEntitySelection = (entityId: string, isMultiSelect = false) => {
    if (isMultiSelect) {
      if (app.state.tree.selectedEntityIds.has(entityId)) {
        app.state.tree.selectedEntityIds.delete(entityId);
      } else {
        app.state.tree.selectedEntityIds.add(entityId);
      }
    } else {
      app.state.tree.selectedEntityIds.clear();
      app.state.tree.selectedEntityIds.add(entityId);
    }
    app.state.tree.lastSelectedEntityId = entityId;
  };

  const selectEntityRange = (fromId: string, toId: string, allIds: string[]) => {
    const fromIndex = allIds.indexOf(fromId);
    const toIndex = allIds.indexOf(toId);

    if (fromIndex === -1 || toIndex === -1) return;

    const startIndex = Math.min(fromIndex, toIndex);
    const endIndex = Math.max(fromIndex, toIndex);

    for (let i = startIndex; i <= endIndex; i++) {
      app.state.tree.selectedEntityIds.add(allIds[i]);
    }
    app.state.tree.lastSelectedEntityId = toId;
  };

  const clearSelection = () => {
    app.state.tree.selectedEntityIds.clear();
    app.state.tree.lastSelectedEntityId = undefined;
  };

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
    const target = e.target as HTMLElement;

    const isMenuRelated = target.closest('[aria-pressed]') || target.closest('[role="menu"]') || target.closest('[role="menuitem"]');

    if (isMenuRelated) {
      return;
    }

    const element = target.closest<HTMLElement>('[data-id]');

    if (!element) {
      return;
    }

    const entityId = element.dataset.id;

    if (entityId && (e.shiftKey || e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      e.stopPropagation();

      if (e.shiftKey && app.state.tree.lastSelectedEntityId) {
        selectEntityRange(app.state.tree.lastSelectedEntityId, entityId, getAllEntityIds());
      } else {
        toggleEntitySelection(entityId, true);
      }
      return;
    }

    // NOTE: 이미 선택된 엔티티를 클릭할 때 일단 선택 해제하지 않음. 여러 개 드래그를 가능하도록 함.
    if (
      entityId &&
      !e.shiftKey &&
      !e.ctrlKey &&
      !e.metaKey &&
      (app.state.tree.selectedEntityIds.size <= 1 || !app.state.tree.selectedEntityIds.has(entityId))
    ) {
      clearSelection();
      toggleEntitySelection(entityId, false);
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

  const handlePointerMove: PointerEventHandler<HTMLDivElement> = (e) => {
    if (!dragging || !tree) {
      if (dragTimeout) {
        clearTimeout(dragTimeout);
        dragTimeout = null;
      }

      return;
    }

    const isPostsPanelVisible = app.state.postsOpen || app.preference.current.postsExpanded === 'open';
    if (dragging.eligible && !isPostsPanelVisible) {
      cancelDragging();
      return;
    }

    if (dragging.eligible) {
      dragging.ghost = {
        x: e.clientX,
        y: e.clientY,
        offsetX: dragging.ghost?.offsetX ?? 0,
        offsetY: dragging.ghost?.offsetY ?? 0,
      };
    } else {
      if (Math.abs(dragging.event.clientX - e.clientX) + Math.abs(dragging.event.clientY - e.clientY) > 10) {
        dragging.eligible = true;
        dragging.element.setPointerCapture(e.pointerId);

        const rect = dragging.element.getBoundingClientRect();
        dragging.ghost = {
          x: dragging.event.clientX,
          y: dragging.event.clientY,
          offsetX: dragging.event.clientX - rect.left,
          offsetY: dragging.event.clientY - rect.top,
        };
      } else {
        return;
      }
    }

    const targetElement =
      document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-id]') ??
      document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[role="tree"]')?.querySelector('& > [data-id]:last-child');
    if (!targetElement) {
      return;
    }

    const entityId = dragging.element.dataset.id;
    const isMultipleDrag = entityId && app.state.tree.selectedEntityIds.has(entityId) && app.state.tree.selectedEntityIds.size > 1;

    let isCycle = false;
    if (isMultipleDrag) {
      for (const selectedId of app.state.tree.selectedEntityIds) {
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

    const relativeY = e.clientY - anchorRect.top;
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
        lowerOrder: [...targetElement.querySelectorAll<HTMLElement>('[data-id]')].at(-1)?.dataset.order,
      };
    } else {
      if (relativeY < anchorRect.height / 2) {
        offsetElement = getPreviousElement(tree, targetElement, '[data-id]');
        dragging.indicator.top = anchorRect.top;

        const thisElement = offsetElement ?? targetElement;
        parentElement = thisElement.closest<HTMLElement>(`[data-id]:not([data-id="${thisElement.dataset.id}"])`);

        dragging.drop = {
          lowerOrder: offsetElement?.dataset.order,
          upperOrder: targetElement.dataset.order,
        };
      } else {
        if (targetElement.dataset.type === 'folder') {
          offsetElement = targetElement;
          dragging.indicator.top = targetRect.top + targetRect.height;

          parentElement = targetElement.closest<HTMLElement>(`[data-id]:not([data-id="${targetElement.dataset.id}"])`);

          dragging.drop = {
            lowerOrder: targetElement.dataset.order,
          };
        } else {
          offsetElement = getNextElement(tree, targetElement, '[data-id]');
          dragging.indicator.top = anchorRect.top + anchorRect.height;

          const thisElement = offsetElement ?? targetElement;
          parentElement = thisElement.closest<HTMLElement>(`[data-id]:not([data-id="${thisElement.dataset.id}"])`);

          dragging.drop = {
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

  const handlePointerUp: PointerEventHandler<HTMLDivElement> = async () => {
    if (!dragging) {
      if (dragTimeout) {
        clearTimeout(dragTimeout);
        dragTimeout = null;
      }

      return;
    }

    const entityId = dragging.element.dataset.id;

    // NOTE: 드래그 없이 클릭했을 때 여러 개 선택되어 있다면 선택 해제하고 클릭한 엔티티만 선택
    if (!dragging.eligible && entityId && app.state.tree.selectedEntityIds.size > 1 && app.state.tree.selectedEntityIds.has(entityId)) {
      clearSelection();
      toggleEntitySelection(entityId, false);
    }

    if (dragging.eligible) {
      on(window, 'click', (e) => e.preventDefault(), { capture: true, once: true });
    }

    if (dragging.drop) {
      const { parentId, lowerOrder, upperOrder } = dragging.drop;

      // NOTE: 기다림 없이 즉시 드래그 해제
      cancelDragging();

      const selectedIds = [...app.state.tree.selectedEntityIds];
      await moveEntities({
        entityIds: selectedIds,
        parentEntityId: parentId ?? null,
        lowerOrder,
        upperOrder,
      });
      mixpanel.track('move_entities', { totalCount: selectedIds.length, parentEntityId: parentId ?? null, lowerOrder, upperOrder });

      return;
    }

    cancelDragging();
  };

  const cancelDragging = () => {
    if (!dragging) {
      return;
    }

    if (dragTimeout) {
      clearTimeout(dragTimeout);
      dragTimeout = null;
    }

    if (dragging.eligible && dragging.element.hasPointerCapture(dragging.event.pointerId)) {
      dragging.element.releasePointerCapture(dragging.event.pointerId);
    }

    dragging = null;
  };

  const draggingEntityCount = $derived.by(() => {
    let count = {
      post: 0,
      canvas: 0,
      folder: 0,
    };

    const entityIds = new Set(app.state.tree.selectedEntityIds);

    const collect = (entities: EntityNode[]) => {
      entities.forEach((entity) => {
        if (entity.node.__typename === 'Folder') {
          if (entityIds.has(entity.id)) {
            count.folder++;
          }

          collect(entity.children as EntityNode[]);
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
      cancelDragging();
    } else {
      e.preventDefault();
      e.stopPropagation();
    }
  }}
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      cancelDragging();
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
    <Entity $entity={entity} $site={_site} />
  {:else}
    <div class={center({ flexGrow: '1' })}>
      <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.disabled' })}>아직 포스트가 없어요</p>
    </div>
  {/each}
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
        zIndex: '50',
      })}
      use:portal
      transition:fade|global={{ duration: 100 }}
    ></div>
  {/key}

  {#if dragging.ghost}
    {@const entityId = dragging.element.dataset.id}
    {@const isMultipleDrag = entityId && app.state.tree.selectedEntityIds.has(entityId) && app.state.tree.selectedEntityIds.size > 1}
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
          zIndex: '[100]',
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
          zIndex: '[100]',
        })}
        use:portal
      >
        <!-- eslint-disable-next-line svelte/no-at-html-tags -->
        {@html dragging.element.outerHTML}
      </div>
    {/if}
  {/if}
{/if}
