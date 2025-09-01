<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getSplitViewContext } from './context.svelte';
  import { getMinSizeForView } from './utils';
  import type { SplitView } from './context.svelte';

  type Props = {
    direction: 'horizontal' | 'vertical';
    index: number;
    view: SplitView;
    containerRef: HTMLDivElement | undefined;
  };

  let { direction, index, view, containerRef }: Props = $props();

  const context = getSplitViewContext();
  let isDragging = $state(false);
  let startPosition = $state(0);
  let rafId: number | null = null;

  // NOTE: 드래그 시작 시점의 각 뷰 크기를 저장
  let initialViewSizes = $state<Record<string, number>>({});

  const handlePointerDown = (e: PointerEvent) => {
    e.preventDefault();
    e.stopPropagation();
    isDragging = true;

    const isHorizontal = direction === 'horizontal';
    startPosition = isHorizontal ? e.clientX : e.clientY;

    if (view.type === 'container') {
      const leftChildId = view.children[index].id;
      const rightChildId = view.children[index + 1].id;

      const currentPercentages = { ...context.state.current.currentPercentages };
      const basePercentages = { ...context.state.current.basePercentages };

      basePercentages[leftChildId] = currentPercentages[leftChildId];
      basePercentages[rightChildId] = currentPercentages[rightChildId];

      // NOTE: 뷰들의 합이 정확히 100%가 되도록 정규화
      let currentSum = 0;
      view.children.forEach((child) => {
        const current = currentPercentages[child.id] || 100 / view.children.length;
        currentSum += current;
      });

      const normalizeFactor = currentSum === 0 ? 1 : 100 / currentSum;

      view.children.forEach((child) => {
        if (!basePercentages[child.id]) {
          basePercentages[child.id] = 100 / view.children.length;
        }
        if (currentPercentages[child.id]) {
          currentPercentages[child.id] = currentPercentages[child.id] * normalizeFactor;
        } else {
          currentPercentages[child.id] = basePercentages[child.id];
        }
        initialViewSizes[child.id] = currentPercentages[child.id];
      });

      context.state.current = {
        ...context.state.current,
        basePercentages,
        currentPercentages,
      };
    }

    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  };

  const updateSizes = (currentPosition: number) => {
    if (!isDragging || view.type !== 'container' || !containerRef) return;

    const rect = containerRef.getBoundingClientRect();
    const isHorizontal = direction === 'horizontal';
    const deltaPixels = currentPosition - startPosition;
    const totalSize = isHorizontal ? rect.width : rect.height;
    if (totalSize <= 0) return;

    const deltaPercent = (deltaPixels / totalSize) * 100;

    const leftChild = view.children[index];
    const rightChild = view.children[index + 1];
    const leftMinSize = getMinSizeForView(leftChild, direction);
    const rightMinSize = getMinSizeForView(rightChild, direction);
    const leftMinSizePercent = (leftMinSize / totalSize) * 100;
    const rightMinSizePercent = (rightMinSize / totalSize) * 100;

    const currentPercentages = { ...context.state.current.currentPercentages };

    const leftChildId = view.children[index].id;
    const rightChildId = view.children[index + 1].id;

    let leftDesired = initialViewSizes[leftChildId] + deltaPercent;
    let rightDesired = initialViewSizes[rightChildId] - deltaPercent;

    // NOTE: 왼쪽 뷰가 최소 크기에 도달했을 때, 추가 공간은 인접하지 않은 뷰들을 압축하여 확보
    if (leftDesired < leftMinSizePercent && deltaPercent < 0) {
      currentPercentages[leftChildId] = leftMinSizePercent;

      const totalAvailable = initialViewSizes[leftChildId] + initialViewSizes[rightChildId] - leftMinSizePercent;
      currentPercentages[rightChildId] = Math.min(totalAvailable, initialViewSizes[rightChildId] - deltaPercent);

      if (rightDesired > totalAvailable) {
        let remainingNeeded = rightDesired - totalAvailable;

        for (let i = index - 1; i >= 0 && remainingNeeded > 0; i--) {
          const childId = view.children[i].id;
          if (childId === leftChildId || childId === rightChildId) continue;

          const initialSize = initialViewSizes[childId];
          const childMinSize = getMinSizeForView(view.children[i], direction);
          const childMinSizePercent = (childMinSize / totalSize) * 100;
          const availableToCompress = Math.max(0, initialSize - childMinSizePercent);
          const compressed = Math.min(availableToCompress, remainingNeeded);

          if (compressed > 0) {
            currentPercentages[childId] = initialSize - compressed;
            currentPercentages[rightChildId] += compressed;
            remainingNeeded -= compressed;
          }
        }
      }
      // NOTE: 오른쪽 뷰가 최소 크기에 도달했을 때
    } else if (rightDesired < rightMinSizePercent && deltaPercent > 0) {
      currentPercentages[rightChildId] = rightMinSizePercent;

      const totalAvailable = initialViewSizes[leftChildId] + initialViewSizes[rightChildId] - rightMinSizePercent;
      currentPercentages[leftChildId] = Math.min(totalAvailable, initialViewSizes[leftChildId] + deltaPercent);

      if (leftDesired > totalAvailable) {
        let remainingNeeded = leftDesired - totalAvailable;

        for (let i = index + 2; i < view.children.length && remainingNeeded > 0; i++) {
          const childId = view.children[i].id;
          if (childId === leftChildId || childId === rightChildId) continue;

          const initialSize = initialViewSizes[childId];
          const childMinSize = getMinSizeForView(view.children[i], direction);
          const childMinSizePercent = (childMinSize / totalSize) * 100;
          const availableToCompress = Math.max(0, initialSize - childMinSizePercent);
          const compressed = Math.min(availableToCompress, remainingNeeded);

          if (compressed > 0) {
            currentPercentages[childId] = initialSize - compressed;
            currentPercentages[leftChildId] += compressed;
            remainingNeeded -= compressed;
          }
        }
      }
    } else {
      currentPercentages[leftChildId] = Math.max(leftMinSizePercent, leftDesired);
      currentPercentages[rightChildId] = Math.max(rightMinSizePercent, rightDesired);
    }

    context.state.current = {
      ...context.state.current,
      currentPercentages,
    };
  };

  const handlePointerMove = (e: PointerEvent) => {
    if (!isDragging) return;

    if (rafId !== null) {
      cancelAnimationFrame(rafId);
    }

    rafId = requestAnimationFrame(() => {
      const isHorizontal = direction === 'horizontal';
      const currentPosition = isHorizontal ? e.clientX : e.clientY;
      updateSizes(currentPosition);
      rafId = null;
    });
  };

  const handlePointerUp = (e: PointerEvent) => {
    if (isDragging && view.type === 'container') {
      const leftChildId = view.children[index].id;
      const rightChildId = view.children[index + 1].id;

      const basePercentages = { ...context.state.current.basePercentages };
      const currentPercentages = context.state.current.currentPercentages;

      basePercentages[leftChildId] = currentPercentages[leftChildId];
      basePercentages[rightChildId] = currentPercentages[rightChildId];

      view.children.forEach((child) => {
        const childId = child.id;
        if (childId === leftChildId || childId === rightChildId) return;

        basePercentages[childId] = currentPercentages[childId];
      });

      context.state.current = {
        ...context.state.current,
        basePercentages,
      };
    }

    cleanupDrag(e);
  };

  const cleanupDrag = (e: PointerEvent) => {
    if (rafId !== null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }

    isDragging = false;
    initialViewSizes = {};
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  };
</script>

<button
  class={css({
    width: direction === 'horizontal' ? '4px' : 'auto',
    height: direction === 'vertical' ? '4px' : 'auto',
    backgroundColor: 'border.subtle',
    flexShrink: '0',
    position: 'relative',
    cursor: direction === 'horizontal' ? 'col-resize' : 'row-resize',
    userSelect: 'none',
    border: 'none',
    padding: '0',
    _hover: {
      backgroundColor: 'border.strong',
    },
  })}
  aria-label="크기 조절"
  onlostpointercapture={cleanupDrag}
  onpointercancel={cleanupDrag}
  onpointerdown={handlePointerDown}
  onpointermovecapture={handlePointerMove}
  onpointerup={handlePointerUp}
  type="button"
>
  <div
    style={direction === 'horizontal' ? 'left: -4px; right: -4px;' : 'top: -4px; bottom: -4px;'}
    class={css({
      position: 'absolute',
      inset: '0',
    })}
  ></div>
</button>
