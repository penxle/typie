<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
  import AlignCenterIcon from '~icons/lucide/align-center';
  import AlignLeftIcon from '~icons/lucide/align-left';
  import AlignRightIcon from '~icons/lucide/align-right';
  import ArrowDownToLineIcon from '~icons/lucide/arrow-down-to-line';
  import ArrowLeftToLineIcon from '~icons/lucide/arrow-left-to-line';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import ArrowUpToLineIcon from '~icons/lucide/arrow-up-to-line';
  import BanIcon from '~icons/lucide/ban';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import EllipsisVerticalIcon from '~icons/lucide/ellipsis-vertical';
  import MoveDownIcon from '~icons/lucide/move-down';
  import MoveLeftIcon from '~icons/lucide/move-left';
  import MoveRightIcon from '~icons/lucide/move-right';
  import MoveUpIcon from '~icons/lucide/move-up';
  import PlusIcon from '~icons/lucide/plus';
  import TableProperties from '~icons/lucide/table-properties';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import type { Editor } from '$lib/editor/editor.svelte';
  import type { TableOverlay as TableOverlayType } from '$lib/editor/slate';

  type Props = {
    editor: Editor;
    overlay: TableOverlayType;
  };

  const MIN_CELL_WIDTH = 40;
  const TABLE_BORDER_WIDTH = 1;
  const TABLE_RESIZE_LIMIT_EPSILON = 0.5;

  const ctx = getEditorContext();
  let { editor, overlay }: Props = $props();

  let resizing = $state<{
    colIndex: number;
    startX: number;
    initialWidths: number[];
    deltaX: number;
  } | null>(null);

  let hoveredPointer = $state<{ x: number; y: number } | null>(null);
  const hoveredColIndex = $derived(hoveredPointer ? findOverlayIndex(overlay.colPositions, hoveredPointer.x) : null);
  const hoveredRowIndex = $derived(hoveredPointer ? findOverlayIndex(overlay.rowPositions, hoveredPointer.y) : null);

  let menuOpenColIndex = $state<number | null>(null);
  let menuOpenRowIndex = $state<number | null>(null);

  let addColButtonHovered = $state(false);
  let addRowButtonHovered = $state(false);
  let addBothButtonHovered = $state(false);
  let tableOverlayRoot: HTMLDivElement | null = null;

  const isLastRowHovered = $derived(
    (hoveredRowIndex !== null && (overlay.startRowIndex ?? 0) + hoveredRowIndex === (overlay.totalRows ?? overlay.rowHeights.length) - 1) ||
      addRowButtonHovered ||
      addBothButtonHovered,
  );
  const isLastColumnHovered = $derived(hoveredColIndex === overlay.colWidthsAsPx.length - 1 || addColButtonHovered || addBothButtonHovered);
  const displayZoom = $derived(editor.layout?.layoutMode.type === 'paginated' ? editor.displayZoom : 1);
  const safeDisplayZoom = $derived(displayZoom > 0 ? displayZoom : 1);
  const inverseDisplayZoom = $derived(1 / safeDisplayZoom);
  const fixedControlTransform = $derived(displayZoom === 1 ? undefined : `scale(${inverseDisplayZoom})`);
  const floatingToolbarOffset = $derived(38 / safeDisplayZoom);
  const addTrackThickness = $derived(23 / safeDisplayZoom);
  const addTrackPadding = $derived(5 / safeDisplayZoom);
  const addButtonSize = $derived(18 / safeDisplayZoom);
  const resizeIndicatorThickness = $derived(4 / safeDisplayZoom);
  const resizeIndicatorHalfThickness = $derived(resizeIndicatorThickness / 2);

  function getVisualColX(colIndex: number, baseX: number): number {
    if (!resizing || resizing.colIndex !== colIndex) {
      return baseX;
    }

    if (colIndex >= resizing.initialWidths.length - 1) {
      const clampedDeltaX = clampProportionResizeDelta(resizing.initialWidths, resizing.deltaX);
      return baseX + clampedDeltaX;
    }

    const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
    const maxDelta = resizing.initialWidths[colIndex + 1] - MIN_CELL_WIDTH;
    const clampedDeltaX = Math.max(minDelta, Math.min(maxDelta, resizing.deltaX));

    return baseX + clampedDeltaX;
  }

  function getContentWidth(): number {
    if ((overlay.contentWidth ?? 0) > 0) {
      return overlay.contentWidth;
    }
    const layoutMode = editor.layout?.layoutMode;
    if (layoutMode?.type === 'paginated') {
      return Math.max(0, layoutMode.pageWidth - layoutMode.pageMarginLeft - layoutMode.pageMarginRight);
    }
    return Math.max(0, layoutMode?.type === 'continuous' ? layoutMode.maxWidth : 0);
  }

  function minTableWidth(colCount: number): number {
    if (colCount <= 0) {
      return 0;
    }
    return MIN_CELL_WIDTH * colCount + TABLE_BORDER_WIDTH * (colCount + 1);
  }

  function clampProportionResizeDelta(initialWidths: number[], deltaX: number): number {
    if (initialWidths.length === 0) {
      return 0;
    }

    const contentWidth = getContentWidth();
    if (contentWidth <= 0) {
      return 0;
    }

    const currentTableWidth = overlay.bounds.width;
    const minWidth = Math.max(minTableWidth(initialWidths.length), overlay.minProportionWidth ?? 0);
    const maxWidth = Math.max(minWidth, overlay.maxProportionWidth ?? contentWidth);

    if (minWidth > maxWidth) {
      return 0;
    }

    const effectiveMinWidth = currentTableWidth <= minWidth + TABLE_RESIZE_LIMIT_EPSILON ? currentTableWidth : minWidth;
    const minDelta = effectiveMinWidth - currentTableWidth;
    const maxDelta = maxWidth - currentTableWidth;
    return clamp(deltaX, minDelta, maxDelta);
  }

  function toRatioWidths(widths: number[]): number[] {
    if (widths.length === 0) {
      return [];
    }

    const safe = widths.map((width) => (Number.isFinite(width) && width > 0 ? width : 0));
    const total = safe.reduce((sum, width) => sum + width, 0);
    if (total <= 0) {
      return widths.map(() => 1 / widths.length);
    }

    return safe.map((width) => width / total);
  }

  function findOverlayIndex(boundaries: number[], value: number): number | null {
    if (boundaries.length === 0 || !Number.isFinite(value)) {
      return null;
    }

    if (value <= 0) {
      return 0;
    }

    let lo = 0;
    let hi = boundaries.length - 1;
    while (lo < hi) {
      const mid = (lo + hi) >> 1;
      if (value < boundaries[mid]) {
        hi = mid;
      } else {
        lo = mid + 1;
      }
    }
    return lo;
  }

  function getColLeft(colIndex: number): number {
    if (colIndex === 0) {
      return 0;
    }
    return overlay.colPositions[colIndex - 1];
  }

  function getColWidth(colIndex: number): number {
    return overlay.colWidthsAsPx[colIndex];
  }

  function getRowTop(rowIndex: number): number {
    if (rowIndex === 0) {
      return 0;
    }
    return overlay.rowPositions[rowIndex - 1];
  }

  function getRowHeight(rowIndex: number): number {
    return overlay.rowHeights[rowIndex];
  }

  function updateHoveredPointerFromClient(clientX: number, clientY: number): void {
    if (!tableOverlayRoot) {
      return;
    }

    const rect = tableOverlayRoot.getBoundingClientRect();
    const localX = (clientX - rect.left) / displayZoom;
    const localY = (clientY - rect.top) / displayZoom;
    hoveredPointer = { x: localX, y: localY };
  }

  function isClientInTableInteractionArea(clientX: number, clientY: number): boolean {
    if (!tableOverlayRoot) {
      return false;
    }

    const targetEl = document.elementFromPoint(clientX, clientY);
    if (targetEl instanceof Node && tableOverlayRoot.contains(targetEl)) {
      return true;
    }

    if (targetEl instanceof Node && !editor.scrollContainerEl?.contains(targetEl)) {
      return false;
    }

    const rect = tableOverlayRoot.getBoundingClientRect();
    return clientX >= rect.left && clientX <= rect.right && clientY >= rect.top && clientY <= rect.bottom;
  }

  function syncHoverFromWindowPointer(clientX: number, clientY: number): void {
    const inside = isClientInTableInteractionArea(clientX, clientY);
    isTableHovered = inside;
    if (!inside) {
      hoveredPointer = null;
      return;
    }

    updateHoveredPointerFromClient(clientX, clientY);
  }

  function handleWindowPointerMove(event: PointerEvent): void {
    syncHoverFromWindowPointer(event.clientX, event.clientY);
  }

  function handleWindowPointerLeave(): void {
    isTableHovered = false;
    hoveredPointer = null;
  }

  let isTableHovered = $state(false);
  let menuOpen = $state(false);
  let buttonHovered = $state(false);

  const isButtonVisible = $derived(isTableHovered || overlay.isFocused || menuOpen || buttonHovered);
  const isAlignButtonVisible = $derived.by(() => {
    if (!Number.isFinite(overlay.proportion)) {
      return true;
    }

    return overlay.proportion < 1 - 0.001;
  });
  const activeColIndex = $derived.by(() => {
    const idx = menuOpenColIndex ?? hoveredColIndex;
    if (idx === null || idx < 0 || idx >= overlay.colWidthsAsPx.length) {
      return null;
    }
    return idx;
  });
  const activeRowIndex = $derived.by(() => {
    const idx = menuOpenRowIndex ?? hoveredRowIndex;
    if (idx === null || idx < 0 || idx >= overlay.rowHeights.length) {
      return null;
    }
    return idx;
  });

  $effect(() => {
    if (menuOpenColIndex !== null || menuOpenRowIndex !== null || menuOpen) {
      editor.inputElement?.blur();
    } else if (ctx.paneFocused) {
      editor.inputElement?.focus();
    }

    return () => {
      // Table 삭제 후 즉시 focus
      if (ctx.paneFocused && (menuOpenColIndex !== null || menuOpenRowIndex !== null || menuOpen)) {
        editor.inputElement?.focus();
      }
    };
  });
</script>

<svelte:window on:pointerleave|capture={handleWindowPointerLeave} on:pointermove|capture={handleWindowPointerMove} />

<div
  bind:this={tableOverlayRoot}
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
  role="presentation"
>
  {#if activeColIndex !== null}
    {@const left = getColLeft(activeColIndex)}
    {@const width = getColWidth(activeColIndex)}
    <div
      style:left="{left}px"
      style:top="0"
      style:width="{width}px"
      class={center({
        position: 'absolute',
        top: '0',
        translate: 'auto',
        translateY: '-1/2',
        height: '18px',
        pointerEvents: 'auto',
        cursor: 'text',
      })}
    >
      <div style:transform={fixedControlTransform} style:transform-origin="center center">
        <Menu
          offset={4}
          onopen={() => {
            menuOpenColIndex = activeColIndex;
            editor.dispatch({ type: 'selectTableColumn', tableId: overlay.tableId, col: activeColIndex });
          }}
          ontransitionend={() => {
            menuOpenColIndex = null;
          }}
          placement="bottom-start"
        >
          {#snippet button({ open })}
            <button
              class={center({
                display: open || activeColIndex !== null ? 'flex' : 'none',
                width: '24px',
                height: '18px',
                backgroundColor: open ? 'interactive.hover' : 'surface.default',
                borderWidth: '1px',
                borderColor: 'border.strong',
                borderRadius: '4px',
                color: open ? 'text.default' : 'text.faint',
                boxShadow: 'small',
                cursor: 'pointer',
                _hover: {
                  backgroundColor: 'interactive.hover',
                  color: 'text.default',
                },
              })}
              aria-pressed={open}
              type="button"
            >
              <Icon icon={EllipsisIcon} size={14} />
            </button>
          {/snippet}

          {#snippet children({ close })}
            {#if activeColIndex > 0}
              <MenuItem
                onclick={() => {
                  close();
                  editor
                    .dispatch({
                      type: 'moveTableColumn',
                      tableId: overlay.tableId,
                      fromCol: activeColIndex,
                      toCol: activeColIndex - 1,
                    })
                    .scrollIntoView();
                  editor.focus();
                }}
              >
                <Icon icon={MoveLeftIcon} size={14} />
                <span>왼쪽으로 이동</span>
              </MenuItem>
            {/if}
            {#if activeColIndex < overlay.colWidthsAsPx.length - 1}
              <MenuItem
                onclick={() => {
                  close();
                  editor
                    .dispatch({
                      type: 'moveTableColumn',
                      tableId: overlay.tableId,
                      fromCol: activeColIndex,
                      toCol: activeColIndex + 1,
                    })
                    .scrollIntoView();
                  editor.focus();
                }}
              >
                <Icon icon={MoveRightIcon} size={14} />
                <span>오른쪽으로 이동</span>
              </MenuItem>
            {/if}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, col: activeColIndex, before: true }).scrollIntoView();
                editor.focus();
              }}
            >
              <Icon icon={ArrowLeftToLineIcon} size={14} />
              <span>왼쪽에 열 추가</span>
            </MenuItem>
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, col: activeColIndex, before: false }).scrollIntoView();
                editor.focus();
              }}
            >
              <Icon icon={ArrowRightToLineIcon} size={14} />
              <span>오른쪽에 열 추가</span>
            </MenuItem>
            <MenuItem
              onclick={() => {
                close();
                if (overlay.colWidthsAsPx.length <= 1) {
                  editor.dispatch({ type: 'deleteNode', nodeId: overlay.tableId }).scrollIntoView();
                } else {
                  editor.dispatch({ type: 'deleteTableColumn', tableId: overlay.tableId, col: activeColIndex }).scrollIntoView();
                }
                editor.focus();
              }}
              variant="danger"
            >
              <Icon icon={Trash2Icon} size={14} />
              <span>{overlay.colWidthsAsPx.length <= 1 ? '테이블 삭제' : '열 삭제'}</span>
            </MenuItem>
          {/snippet}
        </Menu>
      </div>
    </div>
  {/if}

  {#if activeRowIndex !== null}
    {@const top = getRowTop(activeRowIndex)}
    {@const height = getRowHeight(activeRowIndex)}
    {@const globalRowIndex = (overlay.startRowIndex ?? 0) + activeRowIndex}
    <div
      style:left="0"
      style:top="{top}px"
      style:height="{height}px"
      class={center({
        position: 'absolute',
        left: '0',
        translate: 'auto',
        translateX: '-1/2',
        width: '18px',
        pointerEvents: 'auto',
      })}
    >
      <div style:transform={fixedControlTransform} style:transform-origin="center center">
        <Menu
          offset={4}
          onopen={() => {
            menuOpenRowIndex = activeRowIndex;
            editor.dispatch({ type: 'selectTableRow', tableId: overlay.tableId, row: globalRowIndex }).scrollIntoView();
          }}
          ontransitionend={() => {
            menuOpenRowIndex = null;
          }}
          placement="right-start"
        >
          {#snippet button({ open })}
            <button
              class={center({
                display: open || activeRowIndex !== null ? 'flex' : 'none',
                width: '18px',
                height: '24px',
                backgroundColor: open ? 'interactive.hover' : 'surface.default',
                borderWidth: '1px',
                borderColor: 'border.strong',
                borderRadius: '4px',
                color: open ? 'text.default' : 'text.faint',
                boxShadow: 'small',
                cursor: 'pointer',
                _hover: {
                  backgroundColor: 'interactive.hover',
                  color: 'text.default',
                },
              })}
              aria-pressed={open}
              type="button"
            >
              <Icon icon={EllipsisVerticalIcon} size={14} />
            </button>
          {/snippet}

          {#snippet children({ close })}
            {#if globalRowIndex > 0}
              <MenuItem
                onclick={() => {
                  close();
                  editor
                    .dispatch({ type: 'moveTableRow', tableId: overlay.tableId, fromRow: globalRowIndex, toRow: globalRowIndex - 1 })
                    .scrollIntoView();
                  editor.focus();
                }}
              >
                <Icon icon={MoveUpIcon} size={14} />
                <span>위로 이동</span>
              </MenuItem>
            {/if}
            {#if globalRowIndex < (overlay.totalRows ?? overlay.rowHeights.length) - 1}
              <MenuItem
                onclick={() => {
                  close();
                  editor
                    .dispatch({ type: 'moveTableRow', tableId: overlay.tableId, fromRow: globalRowIndex, toRow: globalRowIndex + 1 })
                    .scrollIntoView();
                  editor.focus();
                }}
              >
                <Icon icon={MoveDownIcon} size={14} />
                <span>아래로 이동</span>
              </MenuItem>
            {/if}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, row: globalRowIndex, before: true }).scrollIntoView();
                editor.focus();
              }}
            >
              <Icon icon={ArrowUpToLineIcon} size={14} />
              <span>위에 행 추가</span>
            </MenuItem>
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, row: globalRowIndex, before: false }).scrollIntoView();
                editor.focus();
              }}
            >
              <Icon icon={ArrowDownToLineIcon} size={14} />
              <span>아래에 행 추가</span>
            </MenuItem>
            <MenuItem
              onclick={() => {
                close();
                if ((overlay.totalRows ?? overlay.rowHeights.length) <= 1) {
                  editor.dispatch({ type: 'deleteNode', nodeId: overlay.tableId }).scrollIntoView();
                } else {
                  editor.dispatch({ type: 'deleteTableRow', tableId: overlay.tableId, row: globalRowIndex }).scrollIntoView();
                }
                editor.focus();
              }}
              variant="danger"
            >
              <Icon icon={Trash2Icon} size={14} />
              <span>{(overlay.totalRows ?? overlay.rowHeights.length) <= 1 ? '테이블 삭제' : '행 삭제'}</span>
            </MenuItem>
          {/snippet}
        </Menu>
      </div>
    </div>
  {/if}

  {#each overlay.colPositions as colX, colIndex (colIndex)}
    {@const isLastCol = colIndex === overlay.colWidthsAsPx.length - 1}
    {@const visualX = getVisualColX(colIndex, colX)}
    {@const isResizing = resizing?.colIndex === colIndex}
    <button
      style:left="{visualX - resizeIndicatorHalfThickness}px"
      style:top="0"
      style:width="{resizeIndicatorThickness}px"
      style:height="{overlay.bounds.height}px"
      class={css({
        position: 'absolute',
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
        const initialWidths = [...overlay.colWidthsAsPx];

        resizing = {
          colIndex,
          startX,
          initialWidths,
          deltaX: 0,
        };

        const onMove = (me: PointerEvent) => {
          if (!target.hasPointerCapture(me.pointerId)) return;
          const deltaX = (me.clientX - startX) / displayZoom;
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          resizing = { ...resizing!, deltaX };
        };

        const onUp = (ue: PointerEvent) => {
          target.releasePointerCapture(ue.pointerId);
          target.removeEventListener('pointermove', onMove);
          target.removeEventListener('pointerup', onUp);

          if (resizing) {
            if (colIndex >= resizing.initialWidths.length - 1) {
              const contentWidth = getContentWidth();
              if (contentWidth > 0) {
                const currentTableWidth = overlay.bounds.width;
                const clampedDeltaX = clampProportionResizeDelta(resizing.initialWidths, resizing.deltaX);
                if (Math.abs(clampedDeltaX) <= 0.01) {
                  resizing = null;
                  editor.focus();
                  return;
                }
                const nextTableWidth = currentTableWidth + clampedDeltaX;
                editor.dispatch({
                  type: 'setTableWidth',
                  tableId: overlay.tableId,
                  width: nextTableWidth,
                  contentWidth,
                });
              }
            } else {
              const newWidths = [...resizing.initialWidths];
              const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
              const maxDelta = resizing.initialWidths[colIndex + 1] - MIN_CELL_WIDTH;
              const clampedDeltaX = clamp(resizing.deltaX, minDelta, maxDelta);
              newWidths[colIndex] = resizing.initialWidths[colIndex] + clampedDeltaX;
              newWidths[colIndex + 1] = resizing.initialWidths[colIndex + 1] - clampedDeltaX;

              editor.dispatch({
                type: 'setColumnWidths',
                tableId: overlay.tableId,
                colWidths: toRatioWidths(newWidths),
              });
            }
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
  style:left="{overlay.bounds.x + overlay.bounds.width}px"
  style:top="{overlay.bounds.y}px"
  style:height="{overlay.bounds.height}px"
  style:width="{addTrackThickness}px"
  style:padding-left="{addTrackPadding}px"
  class={css({
    position: 'absolute',
    translate: 'auto',
    pointerEvents: 'auto',
  })}
  data-external-element
  onpointerdown={(e) => e.stopPropagation()}
  onpointerenter={() => (addColButtonHovered = true)}
  onpointerleave={() => (addColButtonHovered = false)}
  role="presentation"
>
  <button
    style:width="{addButtonSize}px"
    class={center({
      height: 'full',
      borderRadius: '4px',
      fontSize: '14px',
      fontWeight: 'medium',
      color: 'text.disabled',
      backgroundColor: 'surface.muted',
      display: isLastColumnHovered ? 'flex' : 'none',
      opacity: '90',
      _hover: {
        display: 'flex',
        backgroundColor: 'interactive.hover',
      },
      _active: {
        color: 'text.bright',
        backgroundColor: 'accent.brand.default',
      },
    })}
    aria-label="열 추가"
    onclick={() => {
      editor
        .dispatch({
          type: 'addTableColumn',
          tableId: overlay.tableId,
          col: overlay.colWidthsAsPx.length - 1,
          before: false,
        })
        .scrollIntoView();
      editor.focus();
    }}
    type="button"
  >
    <span style:display="inline-flex" style:transform={fixedControlTransform} style:transform-origin="center center">
      <Icon icon={PlusIcon} size={14} />
    </span>
  </button>
</div>

<div
  style:left="{overlay.bounds.x}px"
  style:top="{overlay.bounds.y + overlay.bounds.height}px"
  style:width="{overlay.bounds.width}px"
  style:height="{addTrackThickness}px"
  style:padding-top="{addTrackPadding}px"
  class={css({
    position: 'absolute',
    translate: 'auto',
    pointerEvents: 'auto',
  })}
  data-external-element
  onpointerdown={(e) => e.stopPropagation()}
  onpointerenter={() => (addRowButtonHovered = true)}
  onpointerleave={() => (addRowButtonHovered = false)}
  role="presentation"
>
  {#if (overlay.startRowIndex ?? 0) + overlay.rowHeights.length === (overlay.totalRows ?? overlay.rowHeights.length)}
    <button
      style:height="{addButtonSize}px"
      class={center({
        width: 'full',
        borderRadius: '4px',
        fontSize: '14px',
        fontWeight: 'medium',
        color: 'text.disabled',
        backgroundColor: 'surface.muted',
        display: isLastRowHovered ? 'flex' : 'none',
        opacity: '90',
        _hover: {
          display: 'flex',
          backgroundColor: 'interactive.hover',
        },
        _active: {
          color: 'text.bright',
          backgroundColor: 'accent.brand.default',
        },
      })}
      aria-label="행 추가"
      onclick={() => {
        editor
          .dispatch({
            type: 'addTableRow',
            tableId: overlay.tableId,
            row: (overlay.totalRows ?? overlay.rowHeights.length) - 1,
            before: false,
          })
          .scrollIntoView();
        editor.focus();
      }}
      type="button"
    >
      <span style:display="inline-flex" style:transform={fixedControlTransform} style:transform-origin="center center">
        <Icon icon={PlusIcon} size={14} />
      </span>
    </button>
  {/if}
</div>

<div
  style:left="{overlay.bounds.x + overlay.bounds.width}px"
  style:top="{overlay.bounds.y + overlay.bounds.height}px"
  style:width="{addTrackThickness}px"
  style:height="{addTrackThickness}px"
  style:padding-left="{addTrackPadding}px"
  style:padding-top="{addTrackPadding}px"
  class={css({
    position: 'absolute',
    translate: 'auto',
    pointerEvents: 'auto',
  })}
  data-external-element
  onpointerdown={(e) => e.stopPropagation()}
  onpointerenter={() => (addBothButtonHovered = true)}
  onpointerleave={() => (addBothButtonHovered = false)}
  role="presentation"
>
  {#if (overlay.startRowIndex ?? 0) + overlay.rowHeights.length === (overlay.totalRows ?? overlay.rowHeights.length)}
    <button
      style:width="{addButtonSize}px"
      style:height="{addButtonSize}px"
      class={center({
        borderRadius: 'full',
        fontSize: '14px',
        fontWeight: 'medium',
        color: 'text.disabled',
        backgroundColor: 'surface.muted',
        display: isLastRowHovered && isLastColumnHovered ? 'flex' : 'none',
        opacity: '90',
        _hover: {
          display: 'flex',
          backgroundColor: 'interactive.hover',
        },
        _active: {
          color: 'text.bright',
          backgroundColor: 'accent.brand.default',
        },
      })}
      aria-label="행 및 열 추가"
      onclick={() => {
        editor
          .dispatch({
            type: 'addTableRow',
            tableId: overlay.tableId,
            row: (overlay.totalRows ?? overlay.rowHeights.length) - 1,
            before: false,
          })
          .scrollIntoView();
        editor
          .dispatch({
            type: 'addTableColumn',
            tableId: overlay.tableId,
            col: overlay.colWidthsAsPx.length - 1,
            before: false,
          })
          .scrollIntoView();
        editor.focus();
      }}
      type="button"
    >
      <span style:display="inline-flex" style:transform={fixedControlTransform} style:transform-origin="center center">
        <Icon icon={PlusIcon} size={14} />
      </span>
    </button>
  {/if}
</div>

<div
  style:left="{overlay.bounds.x + overlay.bounds.width / 2}px"
  style:top="{overlay.bounds.y - floatingToolbarOffset}px"
  style:transform={fixedControlTransform}
  style:transform-origin="top center"
  class={center({
    position: 'absolute',
    width: 'auto',
    height: '30px',
    display: isButtonVisible ? 'flex' : 'none',
    gap: '2px',
    alignItems: 'center',
    translate: 'auto',
    translateX: '-1/2',
    pointerEvents: 'auto',
    zIndex: '1',
    backgroundColor: 'surface.default',
    borderRadius: '6px',
    boxShadow: 'small',
    borderWidth: '1px',
    borderColor: 'border.strong',
    padding: '2px',
    cursor: 'default',
  })}
  data-external-element
  onpointerdown={(e) => e.stopPropagation()}
  onpointerenter={() => (buttonHovered = true)}
  onpointerleave={() => (buttonHovered = false)}
  role="presentation"
>
  {#if isAlignButtonVisible}
    <Menu offset={4} onopen={() => (menuOpen = true)} ontransitionend={() => (menuOpen = false)} placement="bottom">
      {#snippet button({ open })}
        <button
          class={center({
            display: 'flex',
            fontSize: '14px',
            fontWeight: 'medium',
            color: open ? 'text.default' : 'text.faint',
            backgroundColor: open ? 'interactive.hover' : 'transparent',
            width: '24px',
            height: '24px',
            borderRadius: '4px',
            cursor: 'pointer',
            _hover: {
              backgroundColor: 'interactive.hover',
              color: 'text.default',
            },
          })}
          aria-pressed={open}
          type="button"
        >
          {#if overlay.align === 'center'}
            <Icon icon={AlignCenterIcon} size={14} />
          {:else if overlay.align === 'right'}
            <Icon icon={AlignRightIcon} size={14} />
          {:else}
            <Icon icon={AlignLeftIcon} size={14} />
          {/if}
        </button>
      {/snippet}

      {#snippet children({ close })}
        <MenuItem
          onclick={() => {
            close();
            editor.dispatch({ type: 'setTableAlign', tableId: overlay.tableId, align: 'left' });
            editor.focus();
          }}
        >
          <Icon icon={AlignLeftIcon} size={14} />
          <span>왼쪽 정렬</span>
        </MenuItem>
        <MenuItem
          onclick={() => {
            close();
            editor.dispatch({ type: 'setTableAlign', tableId: overlay.tableId, align: 'center' });
            editor.focus();
          }}
        >
          <Icon icon={AlignCenterIcon} size={14} />
          <span>가운데 정렬</span>
        </MenuItem>
        <MenuItem
          onclick={() => {
            close();
            editor.dispatch({ type: 'setTableAlign', tableId: overlay.tableId, align: 'right' });
            editor.focus();
          }}
        >
          <Icon icon={AlignRightIcon} size={14} />
          <span>오른쪽 정렬</span>
        </MenuItem>
      {/snippet}
    </Menu>
  {/if}

  <Menu offset={4} onopen={() => (menuOpen = true)} ontransitionend={() => (menuOpen = false)} placement="bottom">
    {#snippet button({ open })}
      <button
        class={center({
          display: 'flex',
          fontSize: '14px',
          fontWeight: 'medium',
          color: open ? 'text.default' : 'text.faint',
          backgroundColor: open ? 'interactive.hover' : 'transparent',
          width: '24px',
          height: '24px',
          borderRadius: '4px',
          cursor: 'pointer',
          _hover: {
            backgroundColor: 'interactive.hover',
            color: 'text.default',
          },
        })}
        aria-pressed={open}
        type="button"
      >
        <Icon icon={TableProperties} size={14} />
      </button>
    {/snippet}

    {#snippet children({ close })}
      <MenuItem
        onclick={() => {
          close();
          editor.dispatch({ type: 'setTableBorderStyle', tableId: overlay.tableId, style: 'solid' });
          editor.focus();
        }}
      >
        <div
          class={css({
            width: '14px',
            height: '0',
            borderBottomWidth: '2px',
            borderBottomStyle: 'solid',
            borderColor: 'text.default',
          })}
        ></div>
        <span>실선</span>
      </MenuItem>
      <MenuItem
        onclick={() => {
          close();
          editor.dispatch({ type: 'setTableBorderStyle', tableId: overlay.tableId, style: 'dashed' });
          editor.focus();
        }}
      >
        <div
          class={css({
            width: '14px',
            height: '0',
            borderBottomWidth: '2px',
            borderBottomStyle: 'dashed',
            borderColor: 'text.default',
          })}
        ></div>
        <span>파선</span>
      </MenuItem>
      <MenuItem
        onclick={() => {
          close();
          editor.dispatch({ type: 'setTableBorderStyle', tableId: overlay.tableId, style: 'dotted' });
          editor.focus();
        }}
      >
        <div
          class={css({
            width: '14px',
            height: '0',
            borderBottomWidth: '2px',
            borderBottomStyle: 'dotted',
            borderColor: 'text.default',
          })}
        ></div>
        <span>점선</span>
      </MenuItem>
      <MenuItem
        onclick={() => {
          close();
          editor.dispatch({ type: 'setTableBorderStyle', tableId: overlay.tableId, style: 'none' });
          editor.focus();
        }}
      >
        <Icon icon={BanIcon} size={14} />
        <span>없음</span>
      </MenuItem>
    {/snippet}
  </Menu>
</div>
