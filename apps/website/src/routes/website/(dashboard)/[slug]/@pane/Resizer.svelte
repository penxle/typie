<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getPaneGroup } from './context.svelte';
  import { getMinSizeForMember } from './geometry';
  import { findMemberById } from './tree';
  import type { ResizerInfo } from './geometry';

  type Props = {
    resizer: ResizerInfo;
  };

  let { resizer }: Props = $props();

  const context = getPaneGroup();
  let isDragging = false;
  let dragOffset = 0;
  let startPosition = 0;
  let minOffset = 0;
  let maxOffset = 0;
  let initialFlexes: number[] = [];

  const getAxis = () => {
    if (!context.state.current.root) return null;
    const found = findMemberById(context.state.current.root, resizer.containerId);
    return found?.type === 'axis' ? found : null;
  };

  const handlePointerDown = (e: PointerEvent) => {
    const axis = getAxis();
    if (!axis) return;

    e.preventDefault();
    e.stopPropagation();
    isDragging = true;
    dragOffset = 0;

    const isHorizontal = resizer.direction === 'horizontal';
    startPosition = isHorizontal ? e.clientX : e.clientY;
    initialFlexes = [...axis.flexes];

    const totalSize = resizer.axisSize;
    const totalFlex = initialFlexes.reduce((s, f) => s + f, 0);
    const i = resizer.index;

    let leftCompressible = 0;
    for (let j = 0; j <= i; j++) {
      const childMinFlex = (getMinSizeForMember(axis.children[j], resizer.direction) / totalSize) * totalFlex;
      leftCompressible += initialFlexes[j] - childMinFlex;
    }

    let rightCompressible = 0;
    for (let j = i + 1; j < axis.children.length; j++) {
      const childMinFlex = (getMinSizeForMember(axis.children[j], resizer.direction) / totalSize) * totalFlex;
      rightCompressible += initialFlexes[j] - childMinFlex;
    }

    minOffset = -(leftCompressible / totalFlex) * totalSize;
    maxOffset = (rightCompressible / totalFlex) * totalSize;

    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  };

  const updateSizes = (currentPosition: number) => {
    const axis = getAxis();
    if (!isDragging || !axis) return;

    const deltaPixels = currentPosition - startPosition;
    const totalSize = resizer.axisSize;
    if (totalSize <= 0) return;

    const totalFlex = initialFlexes.reduce((s, f) => s + f, 0);
    const deltaFlex = (deltaPixels / totalSize) * totalFlex;

    const i = resizer.index;
    const newFlexes = [...initialFlexes];

    const leftMinFlex = (getMinSizeForMember(axis.children[i], resizer.direction) / totalSize) * totalFlex;
    const rightMinFlex = (getMinSizeForMember(axis.children[i + 1], resizer.direction) / totalSize) * totalFlex;

    let leftDesired = initialFlexes[i] + deltaFlex;
    let rightDesired = initialFlexes[i + 1] - deltaFlex;

    // NOTE: 왼쪽이 최소 크기에 도달 → 인접하지 않은 왼쪽 pane들을 압축
    if (leftDesired < leftMinFlex && deltaFlex < 0) {
      newFlexes[i] = leftMinFlex;

      const totalAvailable = initialFlexes[i] + initialFlexes[i + 1] - leftMinFlex;
      newFlexes[i + 1] = Math.min(totalAvailable, initialFlexes[i + 1] - deltaFlex);

      if (rightDesired > totalAvailable) {
        let remainingNeeded = rightDesired - totalAvailable;

        for (let j = i - 1; j >= 0 && remainingNeeded > 0; j--) {
          const childMinFlex = (getMinSizeForMember(axis.children[j], resizer.direction) / totalSize) * totalFlex;
          const availableToCompress = Math.max(0, initialFlexes[j] - childMinFlex);
          const compressed = Math.min(availableToCompress, remainingNeeded);

          if (compressed > 0) {
            newFlexes[j] = initialFlexes[j] - compressed;
            newFlexes[i + 1] += compressed;
            remainingNeeded -= compressed;
          }
        }
      }
      // NOTE: 오른쪽이 최소 크기에 도달 → 인접하지 않은 오른쪽 pane들을 압축
    } else if (rightDesired < rightMinFlex && deltaFlex > 0) {
      newFlexes[i + 1] = rightMinFlex;

      const totalAvailable = initialFlexes[i] + initialFlexes[i + 1] - rightMinFlex;
      newFlexes[i] = Math.min(totalAvailable, initialFlexes[i] + deltaFlex);

      if (leftDesired > totalAvailable) {
        let remainingNeeded = leftDesired - totalAvailable;

        for (let j = i + 2; j < axis.children.length && remainingNeeded > 0; j++) {
          const childMinFlex = (getMinSizeForMember(axis.children[j], resizer.direction) / totalSize) * totalFlex;
          const availableToCompress = Math.max(0, initialFlexes[j] - childMinFlex);
          const compressed = Math.min(availableToCompress, remainingNeeded);

          if (compressed > 0) {
            newFlexes[j] = initialFlexes[j] - compressed;
            newFlexes[i] += compressed;
            remainingNeeded -= compressed;
          }
        }
      }
    } else {
      newFlexes[i] = Math.max(leftMinFlex, leftDesired);
      newFlexes[i + 1] = Math.max(rightMinFlex, rightDesired);
    }

    for (const [j, newFlex] of newFlexes.entries()) {
      axis.flexes[j] = newFlex;
    }
  };

  const handlePointerMove = (e: PointerEvent) => {
    if (!isDragging) return;

    const isHorizontal = resizer.direction === 'horizontal';
    const currentPosition = isHorizontal ? e.clientX : e.clientY;
    dragOffset = Math.max(minOffset, Math.min(maxOffset, currentPosition - startPosition));
    context.activeResizer = { rect: resizer.rect, direction: resizer.direction, dragOffset };
  };

  const handlePointerUp = (e: PointerEvent) => {
    if (isDragging && dragOffset !== 0) {
      updateSizes(startPosition + dragOffset);
    }
    cleanupDrag(e);
  };

  const cleanupDrag = (e: PointerEvent) => {
    isDragging = false;
    dragOffset = 0;
    initialFlexes = [];
    context.activeResizer = null;
    (e.currentTarget as HTMLElement).releasePointerCapture(e.pointerId);
  };
</script>

<button
  style:position="absolute"
  style:left="{resizer.rect.left}px"
  style:top="{resizer.rect.top}px"
  style:width="{resizer.rect.width}px"
  style:height="{resizer.rect.height}px"
  class={css({
    backgroundColor: 'border.subtle',
    cursor: resizer.direction === 'horizontal' ? 'col-resize' : 'row-resize',
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
    style={resizer.direction === 'horizontal' ? 'left: -3px; right: -3px;' : 'top: -3px; bottom: -3px;'}
    class={css({
      position: 'absolute',
      inset: '0',
      zIndex: 'overEditor',
    })}
  ></div>
</button>
