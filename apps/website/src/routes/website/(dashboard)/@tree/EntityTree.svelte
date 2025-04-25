<script lang="ts">
  import { on } from 'svelte/events';
  import { fade } from 'svelte/transition';
  import { fragment, graphql } from '$graphql';
  import { portal } from '$lib/actions';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import Entity from './Entity.svelte';
  import { getNextElement, getPreviousElement, maxDepth } from './utils';
  import type { PointerEventHandler } from 'svelte/elements';
  import type { DashboardLayout_EntityTree_site } from '$graphql';

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
          ...DashboardLayout_EntityTree_Entity_entity

          children {
            id
            ...DashboardLayout_EntityTree_Entity_entity

            children {
              id
              ...DashboardLayout_EntityTree_Entity_entity
            }
          }
        }
      }
    `),
  );

  const moveEntity = graphql(`
    mutation DashboardLayout_EntityTree_MoveEntity_Mutation($input: MoveEntityInput!) {
      moveEntity(input: $input) {
        id
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
  };

  let tree = $state<HTMLElement>();
  let dragging = $state<Dragging | null>(null);

  const handlePointerDown: PointerEventHandler<HTMLDivElement> = (e) => {
    const element = (e.target as HTMLElement).closest<HTMLElement>('[data-id]');

    if (!element) {
      return;
    }

    dragging = {
      eligible: false,
      event: e,
      element,
      indicator: {},
    };
  };

  const handlePointerMove: PointerEventHandler<HTMLDivElement> = (e) => {
    if (!dragging || !tree) {
      return;
    }

    if (!dragging.eligible) {
      if (Math.abs(dragging.event.clientX - e.clientX) + Math.abs(dragging.event.clientY - e.clientY) > 10) {
        dragging.eligible = true;
        dragging.element.setPointerCapture(e.pointerId);
      } else {
        return;
      }
    }

    const targetElement = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>('[data-id]');
    if (!targetElement) {
      return;
    }

    const isCycle = dragging.element.contains(targetElement);
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
        offsetElement = getNextElement(tree, targetElement, '[data-id]');
        dragging.indicator.top = anchorRect.top + anchorRect.height;

        const thisElement = offsetElement ?? targetElement;
        parentElement = thisElement.closest<HTMLElement>(`[data-id]:not([data-id="${thisElement.dataset.id}"])`);

        dragging.drop = {
          lowerOrder: targetElement.dataset.order,
          upperOrder: offsetElement?.dataset.order,
        };
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
      const parentDepth = Number(parentElement.dataset.depth ?? 0);
      const draggingDepth = Number(dragging.element.dataset.depth ?? 0);
      const draggingMaxDepth = Math.max(
        draggingDepth,
        ...[...dragging.element.querySelectorAll<HTMLElement>('[data-id]')].map((element) => Number(element.dataset.depth ?? 0)),
      );

      const depthDelta = draggingMaxDepth - draggingDepth + (dragging.element.dataset.type === 'folder' ? 1 : 0);

      if (parentDepth + depthDelta > maxDepth) {
        dragging.indicator = {};
        dragging.drop = undefined;
        return;
      }

      dragging.drop.parentId = parentElement.dataset.id;
    }
  };

  const handlePointerUp: PointerEventHandler<HTMLDivElement> = async () => {
    if (!dragging) {
      return;
    }

    if (dragging.eligible) {
      on(window, 'click', (e) => e.preventDefault(), { capture: true, once: true });
    }

    if (dragging.drop) {
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const entityId = dragging.element.dataset.id!;
      const { parentId, lowerOrder, upperOrder } = dragging.drop;

      await moveEntity({
        entityId,
        parentEntityId: parentId ?? null,
        lowerOrder,
        upperOrder,
      });
    }

    cancelDragging();
  };

  const cancelDragging = () => {
    if (!dragging) {
      return;
    }

    if (dragging.eligible) {
      dragging.element.releasePointerCapture(dragging.event.pointerId);
    }

    dragging = null;
  };
</script>

<svelte:window
  oncontextmenu={() => cancelDragging()}
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      cancelDragging();
    }
  }}
/>

<div
  bind:this={tree}
  class={flex({
    flexDirection: 'column',
    paddingTop: '8px',
    paddingBottom: '48px',
    userSelect: 'none',
    touchAction: 'none',
  })}
  onpointerdowncapture={handlePointerDown}
  onpointermovecapture={handlePointerMove}
  onpointerupcapture={handlePointerUp}
  role="tree"
>
  {#each $site.entities as entity (entity.id)}
    <Entity $entity={entity} />
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
        backgroundColor: 'brand.200',
        pointerEvents: 'none',
        zIndex: '[1000]',
      })}
      use:portal
      transition:fade|global={{ duration: 100 }}
    ></div>
  {/key}
{/if}
