<script lang="ts">
  import { cva } from '@typie/styled-system/css';
  import mixpanel from 'mixpanel-browser';
  import { getSplitViewContext } from './context.svelte';
  import { getDragDropContext } from './drag-context.svelte';
  import type { SplitViewItem } from './context.svelte';
  import type { DropZone } from './drag-context.svelte';

  type Props = {
    viewItem: SplitViewItem;
    viewElement: HTMLElement | undefined;
  };

  let { viewItem, viewElement }: Props = $props();

  const splitView = getSplitViewContext();
  const dragDrop = getDragDropContext();

  let dropZone = $state<DropZone | null>(null);
  const isDragging = $derived(dragDrop.state.isDragging);
  const isActive = $derived(isDragging && dropZone !== null);

  const finalizeDrop = () => {
    dropZone = null;
    dragDrop.endDrag();
  };

  const getDropZone = (mouseX: number, mouseY: number): DropZone | null => {
    if (!viewElement) return null;

    const rect = viewElement.getBoundingClientRect();

    if (mouseX < rect.left || mouseX > rect.right || mouseY < rect.top || mouseY > rect.bottom) {
      return null;
    }

    const x = mouseX - rect.left;
    const y = mouseY - rect.top;
    const width = rect.width;
    const height = rect.height;

    const centerMargin = 0.3;
    const leftBound = width * centerMargin;
    const rightBound = width * (1 - centerMargin);
    const topBound = height * centerMargin;
    const bottomBound = height * (1 - centerMargin);

    if (x < leftBound) return 'left';
    if (x > rightBound) return 'right';
    if (y < topBound) return 'top';
    if (y > bottomBound) return 'bottom';
    return 'center';
  };

  $effect(() => {
    if (!isDragging && !dragDrop.state.droppedItem) {
      dropZone = null;
      return;
    }

    const handleGlobalPointerMove = (e: PointerEvent) => {
      const newZone = getDropZone(e.clientX, e.clientY);
      if (newZone !== dropZone) {
        dropZone = newZone;
      }
    };

    const handleGlobalPointerUp = (e: PointerEvent) => {
      const zone = getDropZone(e.clientX, e.clientY);
      if (!zone) return;

      const droppedItem = dragDrop.state.droppedItem;
      if (!droppedItem || !splitView.state.current.view) return;

      if (droppedItem.type === 'view') {
        const draggedViewId = droppedItem.viewId;

        if (zone === 'center') {
          if (draggedViewId === viewItem.id) {
            finalizeDrop();
            return;
          }

          const success = splitView.swapView(draggedViewId, viewItem.id);
          if (!success) {
            finalizeDrop();
            return;
          }

          mixpanel.track('move_split_view', {
            via: 'drag-drop',
            action: 'replace',
          });
        } else {
          const direction = zone === 'left' || zone === 'right' ? 'horizontal' : 'vertical';
          const position = zone === 'left' || zone === 'top' ? 'before' : 'after';

          const isDuplicate = draggedViewId === viewItem.id;
          const success = splitView.moveView({ viewId: draggedViewId, delete: !isDuplicate }, { viewId: viewItem.id, direction, position });

          if (!success) {
            finalizeDrop();
            return;
          }

          if (isDuplicate) {
            mixpanel.track('duplicate_split_view', {
              via: 'drag-drop',
              direction,
              position,
            });
          } else {
            mixpanel.track('move_split_view', {
              via: 'drag-drop',
              action: 'add',
              direction,
              position,
            });
          }
        }
      } else if (droppedItem.type === 'post' || droppedItem.type === 'canvas') {
        if (zone === 'center') {
          const success = splitView.replaceSplitView(viewItem.id, droppedItem.slug);
          if (!success) {
            finalizeDrop();
            return;
          }

          mixpanel.track('replace_split_view', {
            via: 'drag-drop',
          });
        } else {
          const direction = zone === 'left' || zone === 'right' ? 'horizontal' : 'vertical';
          const position = zone === 'left' || zone === 'top' ? 'before' : 'after';

          const success = splitView.addView(droppedItem.slug, { viewId: viewItem.id, direction, position });
          if (!success) {
            finalizeDrop();
            return;
          }

          mixpanel.track('add_split_view', {
            via: 'drag-drop',
            direction,
            position,
          });
        }
      }

      finalizeDrop();
    };

    window.addEventListener('pointermove', handleGlobalPointerMove);
    window.addEventListener('pointerup', handleGlobalPointerUp);

    return () => {
      window.removeEventListener('pointermove', handleGlobalPointerMove);
      window.removeEventListener('pointerup', handleGlobalPointerUp);
    };
  });

  const overlayStyles = cva({
    base: {
      position: 'absolute',
      backgroundColor: 'surface.dark',
      opacity: '[40]',
      borderRadius: '4px',
      pointerEvents: 'none',
      transition: '[0.1s ease-in-out]',
      zIndex: 'overPanel',
    },
    variants: {
      zone: {
        center: {
          inset: '[20px]',
        },
        left: {
          top: '0',
          left: '0',
          bottom: '0',
          width: '1/2',
        },
        right: {
          top: '0',
          right: '0',
          bottom: '0',
          width: '1/2',
        },
        top: {
          top: '0',
          left: '0',
          right: '0',
          height: '1/2',
        },
        bottom: {
          bottom: '0',
          left: '0',
          right: '0',
          height: '1/2',
        },
      },
    },
  });
</script>

{#if isActive && dropZone}
  <div class={overlayStyles({ zone: dropZone })}></div>
{/if}
