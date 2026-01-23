<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { clamp } from '@typie/ui/utils';
  import type { TableOverlay as TableOverlayType } from '@typie/editor';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
    overlay: TableOverlayType;
  };

  const MIN_CELL_WIDTH = 40;

  let { editor, overlay }: Props = $props();

  let resizing = $state<{
    colIndex: number;
    startX: number;
    initialWidths: number[];
    deltaX: number;
  } | null>(null);

  function getVisualColX(colIndex: number, baseX: number): number {
    if (!resizing || resizing.colIndex !== colIndex) {
      return baseX;
    }

    const isLastCol = colIndex === overlay.colWidths.length - 1;
    let clampedDeltaX = resizing.deltaX;

    if (isLastCol) {
      const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
      clampedDeltaX = Math.max(minDelta, resizing.deltaX);
    } else {
      const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
      const maxDelta = resizing.initialWidths[colIndex + 1] - MIN_CELL_WIDTH;
      clampedDeltaX = Math.max(minDelta, Math.min(maxDelta, resizing.deltaX));
    }

    return baseX + clampedDeltaX;
  }
</script>

<div
  style:left="{overlay.bounds.x}px"
  style:top="{overlay.bounds.y}px"
  style:width="{overlay.bounds.width}px"
  style:height="{overlay.bounds.height}px"
  class={css({
    position: 'absolute',
    pointerEvents: 'none',
  })}
  data-external-element
  data-table-overlay={overlay.tableId}
>
  {#each overlay.colPositions as colX, colIndex (colIndex)}
    {@const isLastCol = colIndex === overlay.colWidths.length - 1}
    {@const visualX = getVisualColX(colIndex, colX)}
    {@const isResizing = resizing?.colIndex === colIndex}
    <button
      style:left="{visualX - 2}px"
      style:top="0"
      style:height="{overlay.bounds.height}px"
      class={css({
        position: 'absolute',
        width: '4px',
        backgroundColor: isResizing ? 'accent.brand.default' : 'transparent',
        cursor: 'col-resize',
        pointerEvents: 'auto',
        opacity: isResizing ? '100' : '0',
        transition: '[opacity 0.15s]',
        _hover: {
          opacity: '100',
          backgroundColor: 'accent.brand.default',
        },
      })}
      aria-label={isLastCol ? '테이블 너비 조절' : '열 너비 조절'}
      data-pointer-capture
      onpointerdown={(e) => {
        e.preventDefault();
        const target = e.currentTarget as HTMLElement;
        target.setPointerCapture(e.pointerId);
        const startX = e.clientX;
        const initialWidths = [...overlay.colWidths];

        resizing = {
          colIndex,
          startX,
          initialWidths,
          deltaX: 0,
        };

        const onMove = (me: PointerEvent) => {
          if (!target.hasPointerCapture(me.pointerId)) return;
          const deltaX = me.clientX - startX;
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          resizing = { ...resizing!, deltaX };
        };

        const onUp = (ue: PointerEvent) => {
          target.releasePointerCapture(ue.pointerId);
          target.removeEventListener('pointermove', onMove);
          target.removeEventListener('pointerup', onUp);

          if (resizing) {
            const newWidths = [...resizing.initialWidths];
            let clampedDeltaX = resizing.deltaX;

            if (isLastCol) {
              const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
              clampedDeltaX = clamp(resizing.deltaX, minDelta, Infinity);
              newWidths[colIndex] = resizing.initialWidths[colIndex] + clampedDeltaX;
            } else {
              const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
              const maxDelta = resizing.initialWidths[colIndex + 1] - MIN_CELL_WIDTH;
              clampedDeltaX = clamp(resizing.deltaX, minDelta, maxDelta);
              newWidths[colIndex] = resizing.initialWidths[colIndex] + clampedDeltaX;
              newWidths[colIndex + 1] = resizing.initialWidths[colIndex + 1] - clampedDeltaX;
            }

            editor.dispatch({ type: 'setColumnWidths', tableId: overlay.tableId, colWidths: newWidths });
          }

          resizing = null;
          editor.focus();
        };

        target.addEventListener('pointermove', onMove);
        target.addEventListener('pointerup', onUp);
      }}
      type="button"
    ></button>
  {/each}
</div>

<div
  style:left="{overlay.bounds.x}px"
  style:top="{overlay.bounds.y}px"
  style:width="{overlay.bounds.width}px"
  style:height="{overlay.bounds.height}px"
  class={css({
    position: 'absolute',
    pointerEvents: 'none',
  })}
  data-external-element
  data-pointer-capture
>
  <button
    style:left="{overlay.bounds.width + 4}px"
    style:top="0"
    style:height="{overlay.bounds.height}px"
    class={css({
      position: 'absolute',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: '20px',
      backgroundColor: 'surface.muted',
      borderRadius: '4px',
      color: 'text.disabled',
      fontSize: '16px',
      pointerEvents: 'auto',
      opacity: '0',
      transition: '[opacity 0.15s]',
      _hover: {
        opacity: '100',
        backgroundColor: 'interactive.hover',
        color: 'text.default',
      },
    })}
    aria-label="열 추가"
    onclick={() => {
      editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, afterCol: overlay.colWidths.length - 1 });
      editor.focus();
    }}
    type="button"
  >
    +
  </button>

  <button
    style:left="0"
    style:top="{overlay.bounds.height + 4}px"
    style:width="{overlay.bounds.width}px"
    class={css({
      position: 'absolute',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '20px',
      backgroundColor: 'surface.muted',
      borderRadius: '4px',
      color: 'text.disabled',
      fontSize: '16px',
      pointerEvents: 'auto',
      opacity: '0',
      transition: '[opacity 0.15s]',
      _hover: {
        opacity: '100',
        backgroundColor: 'interactive.hover',
        color: 'text.default',
      },
    })}
    aria-label="행 추가"
    onclick={() => {
      editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, afterRow: overlay.rowHeights.length - 1 });
      editor.focus();
    }}
    type="button"
  >
    +
  </button>

  <button
    style:left="{overlay.bounds.width + 4}px"
    style:top="{overlay.bounds.height + 4}px"
    class={css({
      position: 'absolute',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      width: '20px',
      height: '20px',
      backgroundColor: 'surface.muted',
      borderRadius: 'full',
      color: 'text.disabled',
      fontSize: '16px',
      pointerEvents: 'auto',
      opacity: '0',
      transition: '[opacity 0.15s]',
      _hover: {
        opacity: '100',
        backgroundColor: 'interactive.hover',
        color: 'text.default',
      },
    })}
    aria-label="행 및 열 추가"
    onclick={() => {
      editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, afterRow: overlay.rowHeights.length - 1 });
      editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, afterCol: overlay.colWidths.length - 1 });
      editor.focus();
    }}
    type="button"
  >
    +
  </button>
</div>
