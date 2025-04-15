<script lang="ts">
  import { graphql } from '$graphql';
  import { css, cx } from '$styled-system/css';
  import { token } from '$styled-system/tokens';
  import PageItem from './PageItem.svelte';
  import type { Dragging, DropTarget, Entity, RootEntity } from './types';

  type Props = {
    entities: Entity[];
    depth?: number;
    parent?: Entity | null;
    siteId: string;
    nodeMap?: Map<HTMLElement, (Entity | RootEntity) & { depth: number }>;
  };

  let {
    entities,
    depth = 0,
    parent = null,
    siteId,
    nodeMap = new Map<HTMLElement, (Entity | RootEntity) & { depth: number }>(),
  }: Props = $props();

  let listEl = $state<HTMLElement>();
  let indicatorEl = $state<HTMLElement>();
  let dragging = $state<Dragging | null>(null);
  let dropTarget = $state<DropTarget | null>(null);

  const maxDepth = 2;

  const moveEntity = graphql(`
    mutation DashboardLayout_PageList_MoveEntity_Mutation($input: MoveEntityInput!) {
      moveEntity(input: $input) {
        id

        children {
          id
          order
        }
      }
    }
  `);

  const registerNode = (node: HTMLElement | undefined, entity: (Entity | RootEntity) & { depth: number }) => {
    if (!node) {
      return;
    }
    nodeMap.set(node, entity);
  };

  $effect(() => {
    if (parent) {
      registerNode(listEl, { ...parent, depth });
    } else {
      registerNode(listEl, { id: null, __typename: 'RootEntity', children: entities, node: null, depth });
    }
  });

  // TODO: 모바일 터치 대응(딜레이 주기)

  const findDeepestDepth = (entity: Entity, depth = 0): number => {
    const children = entity.children;

    if (!children || children.length === 0) {
      return depth;
    }

    return Math.max(...children.map((child) => findDeepestDepth(child, depth + 1)));
  };

  const isDraggingOverTarget = (dropTarget: DropTarget, dragging: Dragging, ignoreAboveLine = false) => {
    if (!entities) return;

    const draggingSiblingIndex = entities.findIndex((item) => item.id === dragging.entity.id);

    const isTargetSameAsDragging = dropTarget.elem && dropTarget.elem === dragging.elem;

    // 드롭 타겟이 드래그 중인 아이템 위/아래에 있는지
    const isAboveDraggingItem = !dropTarget.elem && dropTarget.indicatorPosition === draggingSiblingIndex;
    const isBelowDraggingItem = !dropTarget.elem && dropTarget.indicatorPosition === draggingSiblingIndex + 1;

    return (
      dropTarget &&
      dropTarget.list === listEl &&
      (isTargetSameAsDragging ||
        isBelowDraggingItem || // line indicator (아래)
        (!ignoreAboveLine && isAboveDraggingItem)) // line indicator (위)
    );
  };

  const createGhostEl = (draggingEl: HTMLElement) => {
    const draggingElRect = draggingEl.getBoundingClientRect();
    const ghost = draggingEl.cloneNode(true) as HTMLElement;

    ghost.style.position = 'fixed';
    ghost.style.zIndex = '1000';
    ghost.style.left = `${draggingElRect.left}px`;
    ghost.style.top = `${draggingElRect.top}px`;
    ghost.style.width = `${draggingElRect.width}px`;
    ghost.style.height = `${draggingElRect.height}px`;
    ghost.style.filter = 'brightness(0.7)';
    ghost.style.pointerEvents = 'none';
    ghost.style.display = 'none';
    ghost.style.backgroundColor = token('colors.gray.100');
    ghost.style.borderRadius = '6px';

    document.body.append(ghost);

    return ghost;
  };

  const updateGhostElPosition = (dragging: Dragging, e: PointerEvent) => {
    const draggingElRect = dragging.elem.getBoundingClientRect();

    const offsetX = dragging.event.clientX - draggingElRect.left;
    const offsetY = dragging.event.clientY - draggingElRect.top;

    dragging.ghostEl.style.display = 'block';
    dragging.ghostEl.style.left = `${e.clientX - offsetX}px`;
    dragging.ghostEl.style.top = `${e.clientY - offsetY}px`;
    dragging.ghostEl.style.opacity = dropTarget?.elem && dropTarget.elem !== dragging.elem ? '0.25' : '0.35';
  };

  const updateIndicatorPosition = (dragging: Dragging, dropTarget: DropTarget) => {
    if (!indicatorEl) {
      return;
    }

    if (isDraggingOverTarget(dropTarget, dragging, true) || dropTarget.indicatorPosition === null) {
      indicatorEl.style.display = 'none';
      return;
    }

    indicatorEl.style.display = 'block';
    indicatorEl.style.left = `${dropTarget.list.getBoundingClientRect().left}px`;
    indicatorEl.style.width = `${dropTarget.list.getBoundingClientRect().width}px`;

    const childrenElems = dropTarget.list.querySelectorAll<HTMLElement>(':scope > .dnd-item-folder, :scope > .dnd-item-page');

    if (dropTarget.elem) {
      indicatorEl.style.top = `${dropTarget.list.getBoundingClientRect().top}px`;
    } else {
      if (childrenElems.length === 0) {
        // 리스트가 비어있는 경우 맨 위에 indicator를 표시
        indicatorEl.style.top = `${dropTarget.list.getBoundingClientRect().top}px`;
      } else if (dropTarget.indicatorPosition < childrenElems.length) {
        if (dropTarget.indicatorPosition > 0 && childrenElems.length > 1) {
          // 아이템 사이에 indicator를 표시
          const previousBottom = childrenElems[dropTarget.indicatorPosition - 1].getBoundingClientRect().bottom;
          const nextTop = childrenElems[dropTarget.indicatorPosition].getBoundingClientRect().top;

          indicatorEl.style.top = `${(previousBottom + nextTop) / 2}px`;
        } else {
          // 아이템 위에 indicator를 표시
          indicatorEl.style.top = `${childrenElems[dropTarget.indicatorPosition].getBoundingClientRect().top}px`;
        }
      } else {
        // 마지막 아이템인 경우 그 아래에 indicator를 표시
        // NOTE: at(-1)로 고치면 에러 발생함
        // eslint-disable-next-line unicorn/prefer-at
        indicatorEl.style.top = `${childrenElems[childrenElems.length - 1].getBoundingClientRect().bottom}px`;
      }
    }

    if (dropTarget.elem) {
      indicatorEl.style.height = `${dropTarget.elem.getBoundingClientRect().height}px`;
      indicatorEl.style.opacity = '0.9';
      indicatorEl.style.borderWidth = '1px';
    } else {
      indicatorEl.style.height = '3px';
      indicatorEl.style.opacity = '1';
      indicatorEl.style.borderWidth = '0';
    }
  };

  const onPointerDown = (event: PointerEvent, entity: Entity) => {
    event.stopPropagation();

    let draggingEl;

    if (!entity.node) return;

    if (entity.node.__typename === 'Folder') {
      draggingEl = (event.target as HTMLElement).closest('.dnd-item-folder') as HTMLElement;
    } else {
      draggingEl = (event.target as HTMLElement).closest('.dnd-item-page') as HTMLElement;
    }

    const pointerTarget = draggingEl?.closest('.dnd-list') as HTMLElement;

    if (listEl !== pointerTarget) return;

    dragging = {
      entity,
      elem: draggingEl,
      event,
      ghostEl: createGhostEl(draggingEl),
      pointerId: event.pointerId,
      moved: false,
    };
  };

  const onPointerMove = async (event: PointerEvent) => {
    if (!dragging) return;

    if (!dragging.moved && Math.abs(dragging.event.clientX - event.clientX) + Math.abs(dragging.event.clientY - event.clientY) > 5) {
      dragging.moved = true;
      dragging.elem.setPointerCapture(dragging.pointerId);
    }

    if (!dragging.moved) return;

    if (!dragging.elem.hasPointerCapture(dragging.pointerId)) {
      cancelDragging();
      return;
    }

    updateGhostElPosition(dragging, event);

    let pointerTargetList = document.elementFromPoint(event.clientX, event.clientY)?.closest<HTMLElement>('.dnd-list, .dnd-item-folder');

    if (!pointerTargetList) return;

    let parentId = nodeMap.get(pointerTargetList)?.id;

    const isDropTargetDescendant = pointerTargetList && dragging.elem.contains(pointerTargetList);
    const isValidDropTarget = pointerTargetList && !isDropTargetDescendant;

    if (!isValidDropTarget) {
      dropTarget = null;
      return;
    }

    let indicatorPositionDraft: number | null = null;
    let targetElemDraft: HTMLElement | null = null;

    // 드롭 타겟 리스트 내 포인터의 y 좌표
    const pointerTopInList = event.clientY - pointerTargetList.getBoundingClientRect().top;

    // 드롭 타겟 리스트 내 직계 자식 엘리먼트들
    // line indicator
    const childrenElems = pointerTargetList.querySelectorAll<HTMLElement>(':scope > .dnd-item-folder, :scope > .dnd-item-page');
    // box indicator
    const childrenElems2 = pointerTargetList.querySelectorAll<HTMLElement>(
      ':scope > details > ul > .dnd-item-folder, :scope > details > ul > .dnd-item-page, :scope > details > p',
    );

    if (childrenElems2.length > 0) {
      const mineRect = pointerTargetList.querySelector(':scope > details > summary')?.getBoundingClientRect();
      const emptyRect = pointerTargetList.querySelector(':scope > details > p')?.getBoundingClientRect();

      if (emptyRect) {
        const childTop = emptyRect.top - pointerTargetList.getBoundingClientRect().top;

        // 1/4 지점 ~ 3/4 지점 사이에 있으면 indicator를 item 위에 표시
        if (pointerTopInList < childTop + (emptyRect.height / 4) * 3 && !(pointerTopInList > childTop + emptyRect.height)) {
          // 포인터가 위치한 자식 엘리먼트의 인덱스로 indicator 위치를 결정

          if (parent?.id === pointerTargetList.id) {
            parentId = parent.id;
          }
          targetElemDraft = pointerTargetList;
        }
      }

      if (mineRect) {
        const childTop = mineRect.top - pointerTargetList.getBoundingClientRect().top;

        // 1/4 지점 ~ 3/4 지점 사이에 있으면 indicator를 item 위에 표시
        if (pointerTopInList < childTop + (mineRect.height / 4) * 3 && !(pointerTopInList > childTop + mineRect.height)) {
          // 포인터가 위치한 자식 엘리먼트의 인덱스로 indicator 위치를 결정

          if (parent?.id === pointerTargetList.id) {
            parentId = parent.id;
          }
          targetElemDraft = pointerTargetList;
        }
      }
    }

    if (childrenElems) {
      // 포인터가 위치한 자식 엘리먼트의 인덱스로 indicator 위치를 결정
      for (const [i, child] of childrenElems.entries()) {
        const pageRect = child.querySelector(':scope > .dnd-item-body')?.getBoundingClientRect();
        const folderRect = child.querySelector(':scope > details > .dnd-item-body')?.getBoundingClientRect();

        // 페이지 위아래로 indicator 표시
        if (pageRect) {
          const childTop = pageRect.top - pointerTargetList.getBoundingClientRect().top;

          // 1/4 지점보다 위에 있으면 그 앞에 indicator를 표시
          if (pointerTopInList < childTop + pageRect.height / 4) {
            indicatorPositionDraft = i;
            break;
          } else if (pointerTopInList > childTop + pageRect.height) {
            // 3/4 지점보다 아래에 있으면 그 다음에 indicator를 표시
            indicatorPositionDraft = i + 1;
          }
        }

        // 폴더 위아래로 indicator 표시
        if (folderRect) {
          const childTop = folderRect.top - pointerTargetList.getBoundingClientRect().top;
          // 1/4 지점보다 위에 있으면 그 앞에 indicator를 표시
          if (pointerTopInList < childTop + folderRect.height / 4) {
            indicatorPositionDraft = i;
            break;
          } else if (pointerTopInList > childTop + folderRect.height) {
            // 3/4 지점보다 아래에 있으면 그 다음에 indicator를 표시
            indicatorPositionDraft = i + 1;
          }
        }
      }
    }

    if (indicatorPositionDraft === null) {
      // 마지막 아이템인 경우 그 아래에 indicator를 표시
      indicatorPositionDraft = childrenElems.length;
    }

    const targetListDepth =
      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      nodeMap.get(pointerTargetList)!.depth + (targetElemDraft ? 1 : 0);

    const draggingItemDeepestDepth = findDeepestDepth(dragging.entity);
    const res = draggingItemDeepestDepth === 0 && dragging.entity.node?.__typename === 'Folder' ? 1 : draggingItemDeepestDepth;

    const isMaxDepthExceeded = targetListDepth + res > maxDepth;

    // invalid drop target
    if (isMaxDepthExceeded) {
      indicatorPositionDraft = null;
      targetElemDraft = null;
    }

    dropTarget = {
      list: pointerTargetList,
      parentId,
      indicatorPosition: indicatorPositionDraft,
      elem: targetElemDraft,
    } as DropTarget;

    updateIndicatorPosition(dragging, dropTarget);
  };

  const onPointerUp = async () => {
    if (!dragging) return;

    dragging.elem.releasePointerCapture(dragging.pointerId);

    if (dropTarget && !isDraggingOverTarget(dropTarget, dragging)) {
      if (dropTarget.elem && dragging.elem !== dropTarget.elem) {
        // selection indicator: elem이 있으면 해당 아이템의 children으로 들어감
        const targetItem = nodeMap.get(dropTarget.elem);

        if (!targetItem) return;

        await moveEntity({
          entityId: dragging.entity.id,
          parentEntityId: targetItem.id,
          nextOrder: targetItem.children ? targetItem.children[0]?.order : undefined,
        });

        // eslint-disable-next-line unicorn/no-negated-condition
      } else if (dropTarget.indicatorPosition !== null) {
        // line indicator
        const targetList = nodeMap.get(dropTarget.list);

        let nextOrder = null;
        let previousOrder = null;

        const children = targetList?.children;

        if (dropTarget.indicatorPosition === 0 || !children || children.length === 0) {
          // 맨 앞
          nextOrder = children ? children[0]?.order : undefined;
        } else if (dropTarget.indicatorPosition === children.length) {
          // 맨 뒤
          previousOrder = children.at(-1)?.order;
        } else {
          // 중간
          previousOrder = children[dropTarget.indicatorPosition - 1]?.order;
          nextOrder = children[dropTarget.indicatorPosition]?.order;
        }

        await moveEntity({
          entityId: dragging.entity.id,
          parentEntityId: targetList?.id,
          nextOrder,
          previousOrder,
        });
      } else {
        // invalid drop target
        console.log('invalid drop target');
      }
    }

    dragging?.ghostEl.remove();
    dragging = null;
    dropTarget = null;

    if (indicatorEl) {
      indicatorEl.style.display = 'none';
    }
  };

  const cancelDragging = () => {
    if (!dragging) return;

    document.removeEventListener('pointermove', onPointerMove);
    document.removeEventListener('pointerup', onPointerUp);

    dragging.elem.releasePointerCapture(dragging.pointerId);
    dragging?.ghostEl.remove();
    dragging = null;
    dropTarget = null;

    if (indicatorEl) {
      indicatorEl.style.display = 'none';
    }
  };
</script>

<svelte:window
  oncontextmenu={() => cancelDragging()}
  onkeydown={(e) => {
    if (e.key === 'Escape') {
      cancelDragging();
    }
  }}
  onpointermove={(e) => onPointerMove(e)}
  onpointerup={onPointerUp}
/>

<ul
  bind:this={listEl}
  style:margin-left={depth > 0 ? '16px' : '0'}
  class={cx(
    'dnd-list',
    css(
      {
        display: 'flex',
        flexDirection: 'column',
        gap: '3px',
        paddingY: '3px',
        touchAction: 'none',
      },
      parent === null && {
        paddingTop: '8px',
        paddingBottom: '48px',
      },
    ),
  )}
>
  <div
    bind:this={indicatorEl}
    class={cx(
      'dnd-list-indicator',
      css({
        position: 'fixed',
        zIndex: '[100]',
        top: '0',
        left: '0',
        width: 'full',
        height: '3px',
        borderRadius: '6px',
        backgroundColor: 'brand.500/50',
        display: 'none',
        pointerEvents: 'none',
      }),
    )}
    aria-hidden="true"
  ></div>

  {#each entities as entity (entity.id)}
    <PageItem {depth} {entity} {nodeMap} {onPointerDown} {registerNode} {siteId} />
  {/each}
</ul>
