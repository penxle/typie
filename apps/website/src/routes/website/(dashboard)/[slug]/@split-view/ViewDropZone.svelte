<script lang="ts">
  import { cva } from '@typie/styled-system/css';
  import mixpanel from 'mixpanel-browser';
  import { getSplitViewContext } from './context.svelte';
  import { getDragDropContext } from './drag-context.svelte';
  import { addViewToSplitView, calculateViewPercentages, getParentView, replaceViewInSplitView } from './utils';
  import type { SplitView, SplitViewItem } from './context.svelte';
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

      let result: { splitViews: SplitView; focusedSplitViewId: string } | null = null;

      if (zone === 'center') {
        const newView = replaceViewInSplitView(splitView.state.current.view, viewItem.id, droppedItem.slug);
        splitView.state.current.view = newView;
        splitView.state.current.focusedViewId = viewItem.id;

        mixpanel.track('replace_split_view', {
          via: 'drag-drop',
        });
      } else {
        const direction = zone === 'left' || zone === 'right' ? 'horizontal' : 'vertical';
        const position = zone === 'left' || zone === 'top' ? 'before' : 'after';

        result = addViewToSplitView(splitView.state.current.view, viewItem.id, droppedItem.slug, direction, position);

        splitView.state.current.view = result.splitViews;
        splitView.state.current.focusedViewId = result.focusedSplitViewId;

        const parentView = getParentView(result.splitViews, result.focusedSplitViewId);
        if (parentView && parentView.type === 'container') {
          const newPercentages = calculateViewPercentages(
            parentView,
            result.focusedSplitViewId,
            splitView.state.current.currentPercentages,
          );

          splitView.state.current.currentPercentages = {
            ...splitView.state.current.currentPercentages,
            ...newPercentages,
          };

          splitView.state.current.basePercentages = {
            ...splitView.state.current.basePercentages,
            [result.focusedSplitViewId]: newPercentages[result.focusedSplitViewId],
          };
        }

        mixpanel.track('add_split_view', {
          via: 'drag-drop',
          direction,
          position,
        });
      }

      dropZone = null;
      dragDrop.endDrag();
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
