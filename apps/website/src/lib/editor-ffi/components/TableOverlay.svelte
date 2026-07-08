<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { HorizontalDivider, Icon, Menu, MenuItem, Submenu } from '@typie/ui/components';
  import { getThemeContext } from '@typie/ui/context';
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
  import PaintBucketIcon from '~icons/lucide/paint-bucket';
  import PlusIcon from '~icons/lucide/plus';
  import TableProperties from '~icons/lucide/table-properties';
  import Trash2Icon from '~icons/lucide/trash-2';
  import ToolbarColorGrid from '$lib/components/editor/toolbar/ToolbarColorGrid.svelte';
  import { THEME_COLORS } from '$lib/editor/theme';
  import { values } from '$lib/editor/values';
  import { getEditorContext } from '../editor.svelte';
  import type { Message, TableOverlay as TableOverlayType } from '@typie/editor-ffi/browser';
  import type { ThemeVariant } from '$lib/editor/theme';

  type Props = {
    overlay: TableOverlayType;
    readOnly?: boolean;
  };

  const MIN_CELL_WIDTH = 40;
  const TABLE_BORDER_WIDTH = 1;
  const TABLE_RESIZE_LIMIT_EPSILON = 0.5;

  const { editor } = getEditorContext();
  const theme = getThemeContext();
  let { overlay, readOnly = false }: Props = $props();

  const themeVariant = $derived(
    (theme.effectiveTheme === 'light' ? `light-${theme.lightVariant}` : `dark-${theme.darkVariant}`) as ThemeVariant,
  );
  const tc = $derived(THEME_COLORS[themeVariant]);
  const cellBackgroundColors = $derived(
    values.textBackgroundColor.map((c) => ({ label: c.label, value: c.value, color: c.themeKey ? tc[c.themeKey] : null })),
  );

  let resizing = $state<{
    colIndex: number;
    startX: number;
    initialWidths: number[];
    deltaX: number;
  } | null>(null);

  let hoveredPointer = $state<{ x: number; y: number } | null>(null);
  const hoveredColIndex = $derived(
    hoveredPointer
      ? findOverlayIndex(
          overlay.columns.map((col) => col.position),
          hoveredPointer.x,
        )
      : null,
  );
  const hoveredRowIndex = $derived(
    hoveredPointer
      ? findOverlayIndex(
          overlay.rows.map((row) => row.position),
          hoveredPointer.y,
        )
      : null,
  );

  let menuOpenColIndex = $state<number | null>(null);
  let menuOpenRowIndex = $state<number | null>(null);

  let addColButtonHovered = $state(false);
  let addRowButtonHovered = $state(false);
  let addBothButtonHovered = $state(false);
  let tableOverlayRoot = $state<HTMLDivElement | null>(null);

  const rowCount = $derived(overlay.row_count);
  const lastColumnIndex = $derived(overlay.columns.at(-1)?.index ?? 0);
  const isLastRowFragment = $derived(overlay.is_last_row_fragment);
  const isLastRowHovered = $derived(
    (hoveredRowIndex !== null && overlay.rows[hoveredRowIndex]?.index === rowCount - 1) || addRowButtonHovered || addBothButtonHovered,
  );
  const isLastColumnHovered = $derived(hoveredColIndex === overlay.columns.length - 1 || addColButtonHovered || addBothButtonHovered);
  const safeDisplayZoom = $derived.by(() => {
    if (!editor) return 1;
    return editor.safeDisplayZoom();
  });
  const inverseDisplayZoom = $derived(1 / safeDisplayZoom);
  const fixedControlTransform = $derived(safeDisplayZoom === 1 ? undefined : `scale(${inverseDisplayZoom})`);

  const addTrackThickness = $derived(23 / safeDisplayZoom);
  const addTrackPadding = $derived(5 / safeDisplayZoom);
  const addButtonSize = $derived(18 / safeDisplayZoom);
  const floatingToolbarOffset = $derived(38 / safeDisplayZoom);
  const resizeIndicatorThickness = $derived(4 / safeDisplayZoom);
  const resizeIndicatorHalfThickness = $derived(resizeIndicatorThickness / 2);

  function getVisualColX(colIndex: number, baseX: number): number {
    if (!resizing || resizing.colIndex !== colIndex) return baseX;

    if (colIndex >= resizing.initialWidths.length - 1) {
      return baseX + clampProportionResizeDelta(resizing.initialWidths, resizing.deltaX);
    }

    const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
    const maxDelta = resizing.initialWidths[colIndex + 1] - MIN_CELL_WIDTH;
    return baseX + Math.max(minDelta, Math.min(maxDelta, resizing.deltaX));
  }

  function minTableWidth(colCount: number): number {
    return colCount <= 0 ? 0 : MIN_CELL_WIDTH * colCount + TABLE_BORDER_WIDTH * (colCount + 1);
  }

  function clampProportionResizeDelta(initialWidths: number[], deltaX: number): number {
    if (initialWidths.length === 0 || overlay.content_width <= 0) return 0;

    const currentTableWidth = overlay.bounds.width;
    const minWidth = Math.max(minTableWidth(initialWidths.length), 0);
    const maxWidth = Math.max(minWidth, overlay.content_width);

    const effectiveMinWidth = currentTableWidth <= minWidth + TABLE_RESIZE_LIMIT_EPSILON ? currentTableWidth : minWidth;
    return clamp(deltaX, effectiveMinWidth - currentTableWidth, maxWidth - currentTableWidth);
  }

  function findOverlayIndex(boundaries: number[], value: number): number | null {
    if (boundaries.length === 0 || !Number.isFinite(value)) return null;
    if (value <= 0) return 0;

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

  function clampOverlayIndex(index: number | null, length: number): number | null {
    if (index === null || length <= 0) return null;
    return clamp(index, 0, length - 1);
  }

  function getColLeft(colIndex: number): number {
    return colIndex === 0 ? 0 : (overlay.columns[colIndex - 1]?.position ?? 0);
  }

  function getColWidth(colIndex: number): number {
    return overlay.columns[colIndex]?.width_as_px ?? 0;
  }

  function getRowTop(rowIndex: number): number {
    return rowIndex === 0 ? 0 : (overlay.rows[rowIndex - 1]?.position ?? 0);
  }

  function getRowHeight(rowIndex: number): number {
    return overlay.rows[rowIndex]?.height ?? 0;
  }

  function updateHoveredPointerFromClient(clientX: number, clientY: number): void {
    if (!tableOverlayRoot) return;
    const rect = tableOverlayRoot.getBoundingClientRect();
    hoveredPointer = { x: (clientX - rect.left) / safeDisplayZoom, y: (clientY - rect.top) / safeDisplayZoom };
  }

  function isClientInTableInteractionArea(clientX: number, clientY: number): boolean {
    if (!tableOverlayRoot) return false;
    const targetEl = document.elementFromPoint(clientX, clientY);
    if (targetEl instanceof Node && tableOverlayRoot.contains(targetEl)) return true;
    if (targetEl instanceof Node && !editor?.scrollContainerEl?.contains(targetEl)) return false;
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
    lastWindowPointer = { clientX: event.clientX, clientY: event.clientY };
    syncHoverFromWindowPointer(event.clientX, event.clientY);
  }

  function handleWindowPointerLeave(): void {
    isTableHovered = false;
    hoveredPointer = null;
    lastWindowPointer = null;
  }

  let isTableHovered = $state(false);
  let menuOpen = $state(false);
  let cellBgMenuOpen = $state(false);
  let buttonHovered = $state(false);
  let lastWindowPointer = $state<{ clientX: number; clientY: number } | null>(null);

  const isActive = $derived(isTableHovered || overlay.is_focused || menuOpen || cellBgMenuOpen || buttonHovered || resizing !== null);
  const cellSelectionButtonPosition = $derived.by(() => {
    const selection = overlay.cell_selection;
    if (!selection) return null;

    const firstVisibleRow = overlay.rows.at(0)?.index;
    const lastVisibleRow = overlay.rows.at(-1)?.index;
    if (firstVisibleRow === undefined || lastVisibleRow === undefined) return null;

    const colStart = Math.min(selection.anchor_col, selection.head_col);
    const colEnd = Math.max(selection.anchor_col, selection.head_col);
    const rowStart = Math.min(selection.anchor_row, selection.head_row);
    const rowEnd = Math.max(selection.anchor_row, selection.head_row);
    if (rowStart > lastVisibleRow || rowEnd < firstVisibleRow) return null;

    const colStartIndex = overlay.columns.findIndex((column) => column.index === colStart);
    const colEndIndex = overlay.columns.findIndex((column) => column.index === colEnd);
    const rowEndIndex = overlay.rows.findIndex((row) => row.index === Math.min(rowEnd, lastVisibleRow));
    if (colStartIndex === -1 || colEndIndex === -1 || rowEndIndex === -1) return null;

    const leftEdge = getColLeft(colStartIndex);
    const rightEdge = overlay.columns[colEndIndex]?.position ?? overlay.bounds.width;
    const centerX = (leftEdge + rightEdge) / 2;
    const bottomY = overlay.rows[rowEndIndex]?.position ?? overlay.bounds.height;
    return { left: centerX, top: bottomY + 4 };
  });

  const isAlignButtonVisible = $derived(!Number.isFinite(overlay.proportion) || overlay.proportion < 1 - 0.001);

  const activeColIndex = $derived(
    isActive ? clampOverlayIndex(menuOpenColIndex ?? hoveredColIndex ?? overlay.focused_col_index ?? null, overlay.columns.length) : null,
  );
  const activeRowIndex = $derived(
    isActive ? clampOverlayIndex(menuOpenRowIndex ?? hoveredRowIndex ?? overlay.focused_row_index ?? null, overlay.rows.length) : null,
  );

  $effect(() => {
    if (readOnly) return;
    const onMove = (e: PointerEvent) => handleWindowPointerMove(e);
    const onLeave = () => handleWindowPointerLeave();
    window.addEventListener('pointermove', onMove, { capture: true });
    window.addEventListener('pointerleave', onLeave, { capture: true });
    return () => {
      window.removeEventListener('pointermove', onMove, { capture: true });
      window.removeEventListener('pointerleave', onLeave, { capture: true });
    };
  });

  $effect(() => {
    const pointer = lastWindowPointer;
    const root = tableOverlayRoot;
    const overlayRevision = `${overlay.bounds.x}:${overlay.bounds.y}:${overlay.bounds.width}:${overlay.bounds.height}:${overlay.columns.length}:${overlay.rows.length}:${menuOpenColIndex ?? -1}:${menuOpenRowIndex ?? -1}:${safeDisplayZoom}`;

    if (!pointer || !root || overlayRevision.length === 0) return;

    const frame = requestAnimationFrame(() => {
      syncHoverFromWindowPointer(pointer.clientX, pointer.clientY);
    });

    return () => cancelAnimationFrame(frame);
  });

  function enqueueTableOp(op: Message) {
    editor?.enqueue(op);
    editor?.focus();
  }

  function insertAxis(axis: 'horizontal' | 'vertical', index: number, before: boolean) {
    enqueueTableOp({
      type: 'node',
      op: { type: 'table', id: overlay.table_id, op: { type: 'insert_axis', axis, index, before } },
    });
  }

  function deleteAxis(axis: 'horizontal' | 'vertical', index: number) {
    enqueueTableOp({
      type: 'node',
      op: { type: 'table', id: overlay.table_id, op: { type: 'delete_axis', axis, index } },
    });
  }

  function moveAxis(axis: 'horizontal' | 'vertical', from: number, to: number) {
    enqueueTableOp({
      type: 'node',
      op: { type: 'table', id: overlay.table_id, op: { type: 'move_axis', axis, from, to } },
    });
  }

  function setColumnWidths(colWidths: number[]) {
    enqueueTableOp({
      type: 'node',
      op: { type: 'table', id: overlay.table_id, op: { type: 'set_column_widths', widths: colWidths } },
    });
  }

  function setProportion(proportion: number) {
    enqueueTableOp({
      type: 'node',
      op: { type: 'table', id: overlay.table_id, op: { type: 'set_proportion', proportion: Math.round(proportion * 100) } },
    });
  }

  function setBorderStyle(style: 'solid' | 'dashed' | 'dotted' | 'none') {
    enqueueTableOp({
      type: 'node',
      op: { type: 'table', id: overlay.table_id, op: { type: 'set_border_style', border_style: style } },
    });
  }

  function setAlign(align: 'left' | 'center' | 'right') {
    enqueueTableOp({
      type: 'modifier',
      op: { type: 'set_on_node', id: overlay.table_id, modifier: { type: 'alignment', value: align } },
    });
  }
</script>

{#if !readOnly}
  <div
    bind:this={tableOverlayRoot}
    style:left="{overlay.bounds.x}px"
    style:top="{overlay.bounds.y}px"
    style:width="{overlay.bounds.width}px"
    style:height="{overlay.bounds.height}px"
    class={css({ position: 'absolute', pointerEvents: 'none' })}
    role="presentation"
  >
    <!-- Column handle -->
    {#if activeColIndex !== null}
      {@const activeColumn = overlay.columns[activeColIndex]}
      {@const colIndex = activeColumn.index}
      {@const left = getColLeft(activeColIndex)}
      {@const width = getColWidth(activeColIndex)}
      <div
        style:left="{left}px"
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
        onpointerdown={(e) => e.stopPropagation()}
        role="presentation"
      >
        <div style:transform={fixedControlTransform} style:transform-origin="center center">
          <Menu
            offset={4}
            onopen={() => {
              menuOpenColIndex = activeColIndex;
              editor?.enqueue({
                type: 'node',
                op: {
                  type: 'table',
                  id: overlay.table_id,
                  op: { type: 'select_axis', axis: 'vertical', index: colIndex },
                },
              });
            }}
            ontransitionend={() => (menuOpenColIndex = null)}
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
                  _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
                })}
                aria-pressed={open}
                type="button"
              >
                <Icon icon={EllipsisIcon} size={14} />
              </button>
            {/snippet}
            {#snippet children({ close })}
              {#if colIndex > 0}
                <MenuItem
                  onclick={() => {
                    close();
                    moveAxis('vertical', colIndex, colIndex - 1);
                  }}
                >
                  <Icon icon={MoveLeftIcon} size={14} />
                  <span>왼쪽으로 이동</span>
                </MenuItem>
              {/if}
              {#if colIndex < lastColumnIndex}
                <MenuItem
                  onclick={() => {
                    close();
                    moveAxis('vertical', colIndex, colIndex + 1);
                  }}
                >
                  <Icon icon={MoveRightIcon} size={14} />
                  <span>오른쪽으로 이동</span>
                </MenuItem>
              {/if}
              <MenuItem
                onclick={() => {
                  close();
                  insertAxis('vertical', colIndex, true);
                }}
              >
                <Icon icon={ArrowLeftToLineIcon} size={14} />
                <span>왼쪽에 열 추가</span>
              </MenuItem>
              <MenuItem
                onclick={() => {
                  close();
                  insertAxis('vertical', colIndex, false);
                }}
              >
                <Icon icon={ArrowRightToLineIcon} size={14} />
                <span>오른쪽에 열 추가</span>
              </MenuItem>
              <HorizontalDivider />
              <Submenu icon={PaintBucketIcon} label="배경색 설정" listStyle={css.raw({ minWidth: 'auto', padding: '0' })}>
                <li>
                  <ToolbarColorGrid
                    columns={8}
                    currentValue={activeColumn.background_color ?? 'none'}
                    items={cellBackgroundColors}
                    onClose={close}
                    onSelect={(value) => {
                      close();
                      enqueueTableOp({
                        type: 'node',
                        op: {
                          type: 'table',
                          id: overlay.table_id,
                          op: {
                            type: 'set_axis_background_color',
                            axis: 'vertical',
                            index: colIndex,
                            color: value === 'none' ? undefined : value,
                          },
                        },
                      });
                    }}
                  />
                </li>
              </Submenu>
              <HorizontalDivider />
              <MenuItem
                onclick={() => {
                  close();
                  if (overlay.columns.length <= 1) {
                    enqueueTableOp({ type: 'node', op: { type: 'delete', id: overlay.table_id } });
                  } else {
                    deleteAxis('vertical', colIndex);
                  }
                }}
                variant="danger"
              >
                <Icon icon={Trash2Icon} size={14} />
                <span>{overlay.columns.length <= 1 ? '테이블 삭제' : '열 삭제'}</span>
              </MenuItem>
            {/snippet}
          </Menu>
        </div>
      </div>
    {/if}

    <!-- Row handle -->
    {#if activeRowIndex !== null}
      {@const top = getRowTop(activeRowIndex)}
      {@const height = getRowHeight(activeRowIndex)}
      {@const activeRow = overlay.rows[activeRowIndex]}
      {@const rowIndex = activeRow.index}
      <div
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
        onpointerdown={(e) => e.stopPropagation()}
        role="presentation"
      >
        <div style:transform={fixedControlTransform} style:transform-origin="center center">
          <Menu
            offset={4}
            onopen={() => {
              menuOpenRowIndex = activeRowIndex;
              editor?.enqueue({
                type: 'node',
                op: {
                  type: 'table',
                  id: overlay.table_id,
                  op: { type: 'select_axis', axis: 'horizontal', index: rowIndex },
                },
              });
            }}
            ontransitionend={() => (menuOpenRowIndex = null)}
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
                  _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
                })}
                aria-pressed={open}
                type="button"
              >
                <Icon icon={EllipsisVerticalIcon} size={14} />
              </button>
            {/snippet}
            {#snippet children({ close })}
              {#if rowIndex > 0}
                <MenuItem
                  onclick={() => {
                    close();
                    moveAxis('horizontal', rowIndex, rowIndex - 1);
                  }}
                >
                  <Icon icon={MoveUpIcon} size={14} />
                  <span>위로 이동</span>
                </MenuItem>
              {/if}
              {#if rowIndex < rowCount - 1}
                <MenuItem
                  onclick={() => {
                    close();
                    moveAxis('horizontal', rowIndex, rowIndex + 1);
                  }}
                >
                  <Icon icon={MoveDownIcon} size={14} />
                  <span>아래로 이동</span>
                </MenuItem>
              {/if}
              <MenuItem
                onclick={() => {
                  close();
                  insertAxis('horizontal', rowIndex, true);
                }}
              >
                <Icon icon={ArrowUpToLineIcon} size={14} />
                <span>위에 행 추가</span>
              </MenuItem>
              <MenuItem
                onclick={() => {
                  close();
                  insertAxis('horizontal', rowIndex, false);
                }}
              >
                <Icon icon={ArrowDownToLineIcon} size={14} />
                <span>아래에 행 추가</span>
              </MenuItem>
              <HorizontalDivider />
              <Submenu icon={PaintBucketIcon} label="배경색 설정" listStyle={css.raw({ minWidth: 'auto', padding: '0' })}>
                <li>
                  <ToolbarColorGrid
                    columns={8}
                    currentValue={activeRow.background_color ?? 'none'}
                    items={cellBackgroundColors}
                    onClose={close}
                    onSelect={(value) => {
                      close();
                      enqueueTableOp({
                        type: 'node',
                        op: {
                          type: 'table',
                          id: overlay.table_id,
                          op: {
                            type: 'set_axis_background_color',
                            axis: 'horizontal',
                            index: rowIndex,
                            color: value === 'none' ? undefined : value,
                          },
                        },
                      });
                    }}
                  />
                </li>
              </Submenu>
              <HorizontalDivider />
              <MenuItem
                onclick={() => {
                  close();
                  if (rowCount <= 1) {
                    enqueueTableOp({ type: 'node', op: { type: 'delete', id: overlay.table_id } });
                  } else {
                    deleteAxis('horizontal', rowIndex);
                  }
                }}
                variant="danger"
              >
                <Icon icon={Trash2Icon} size={14} />
                <span>{rowCount <= 1 ? '테이블 삭제' : '행 삭제'}</span>
              </MenuItem>
            {/snippet}
          </Menu>
        </div>
      </div>
    {/if}

    <!-- Column resize handles -->
    {#each overlay.columns as col, colIndex (col.index)}
      {@const colX = col.position}
      {@const isLastCol = colIndex === overlay.columns.length - 1}
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
          pointerEvents: isActive ? 'auto' : 'none',
          opacity: isResizing ? '100' : '0',
          transition: '[opacity 0.15s]',
          _hover: { opacity: '100', backgroundColor: 'accent.brand.default' },
        })}
        aria-label={isLastCol ? '테이블 너비 조절' : '열 너비 조절'}
        onpointerdown={(e) => {
          e.preventDefault();
          e.stopPropagation();
          const target = e.currentTarget as HTMLElement;
          target.setPointerCapture(e.pointerId);
          const startX = e.clientX;
          const initialWidths = overlay.columns.map((column) => column.width_as_px);

          resizing = { colIndex, startX, initialWidths, deltaX: 0 };

          const onMove = (me: PointerEvent) => {
            if (!target.hasPointerCapture(me.pointerId)) return;
            const deltaX = (me.clientX - startX) / safeDisplayZoom;
            // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
            resizing = { ...resizing!, deltaX };
          };

          const onUp = (ue: PointerEvent) => {
            target.releasePointerCapture(ue.pointerId);
            target.removeEventListener('pointermove', onMove);
            target.removeEventListener('pointerup', onUp);
            target.removeEventListener('pointercancel', onUp);

            if (resizing) {
              if (colIndex >= resizing.initialWidths.length - 1) {
                if (overlay.content_width > 0) {
                  const clampedDelta = clampProportionResizeDelta(resizing.initialWidths, resizing.deltaX);
                  if (Math.abs(clampedDelta) > 0.01) {
                    const newWidth = overlay.bounds.width + clampedDelta;
                    setProportion(newWidth / overlay.content_width);
                  }
                }
              } else {
                const newWidths = [...resizing.initialWidths];
                const minDelta = MIN_CELL_WIDTH - resizing.initialWidths[colIndex];
                const maxDelta = resizing.initialWidths[colIndex + 1] - MIN_CELL_WIDTH;
                const clampedDelta = clamp(resizing.deltaX, minDelta, maxDelta);
                newWidths[colIndex] = resizing.initialWidths[colIndex] + clampedDelta;
                newWidths[colIndex + 1] = resizing.initialWidths[colIndex + 1] - clampedDelta;
                setColumnWidths(newWidths);
              }
            }

            resizing = null;
            editor?.focus();
          };

          target.addEventListener('pointermove', onMove);
          target.addEventListener('pointerup', onUp);
          target.addEventListener('pointercancel', onUp);
        }}
        type="button"
      ></button>
    {/each}
  </div>

  <!-- Add column button (right edge) -->
  <div
    style:left="{overlay.bounds.x + overlay.bounds.width}px"
    style:top="{overlay.bounds.y}px"
    style:height="{overlay.bounds.height}px"
    style:width="{addTrackThickness}px"
    style:padding-left="{addTrackPadding}px"
    class={css({ position: 'absolute', pointerEvents: isActive ? 'auto' : 'none' })}
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
        color: 'text.disabled',
        backgroundColor: 'surface.muted',
        display: isLastColumnHovered ? 'flex' : 'none',
        opacity: '90',
        _hover: { display: 'flex', backgroundColor: 'interactive.hover' },
        _active: { color: 'text.bright', backgroundColor: 'accent.brand.default' },
      })}
      aria-label="열 추가"
      onclick={() => insertAxis('vertical', lastColumnIndex, false)}
      type="button"
    >
      <span style:display="inline-flex" style:transform={fixedControlTransform} style:transform-origin="center center">
        <Icon icon={PlusIcon} size={14} />
      </span>
    </button>
  </div>

  {#if isLastRowFragment}
    <!-- Add row button (bottom edge) -->
    <div
      style:left="{overlay.bounds.x}px"
      style:top="{overlay.bounds.y + overlay.bounds.height}px"
      style:width="{overlay.bounds.width}px"
      style:height="{addTrackThickness}px"
      style:padding-top="{addTrackPadding}px"
      class={css({ position: 'absolute', pointerEvents: isActive ? 'auto' : 'none' })}
      onpointerdown={(e) => e.stopPropagation()}
      onpointerenter={() => (addRowButtonHovered = true)}
      onpointerleave={() => (addRowButtonHovered = false)}
      role="presentation"
    >
      <button
        style:height="{addButtonSize}px"
        class={center({
          width: 'full',
          borderRadius: '4px',
          color: 'text.disabled',
          backgroundColor: 'surface.muted',
          display: isLastRowHovered ? 'flex' : 'none',
          opacity: '90',
          _hover: { display: 'flex', backgroundColor: 'interactive.hover' },
          _active: { color: 'text.bright', backgroundColor: 'accent.brand.default' },
        })}
        aria-label="행 추가"
        onclick={() => insertAxis('horizontal', rowCount - 1, false)}
        type="button"
      >
        <span style:display="inline-flex" style:transform={fixedControlTransform} style:transform-origin="center center">
          <Icon icon={PlusIcon} size={14} />
        </span>
      </button>
    </div>
  {/if}

  <!-- Corner add button (both row and column) -->
  {#if isLastRowFragment}
    <div
      style:left="{overlay.bounds.x + overlay.bounds.width}px"
      style:top="{overlay.bounds.y + overlay.bounds.height}px"
      style:width="{addTrackThickness}px"
      style:height="{addTrackThickness}px"
      style:padding-left="{addTrackPadding}px"
      style:padding-top="{addTrackPadding}px"
      class={css({ position: 'absolute', pointerEvents: isActive ? 'auto' : 'none' })}
      onpointerdown={(e) => e.stopPropagation()}
      onpointerenter={() => (addBothButtonHovered = true)}
      onpointerleave={() => (addBothButtonHovered = false)}
      role="presentation"
    >
      <button
        style:width="{addButtonSize}px"
        style:height="{addButtonSize}px"
        class={center({
          borderRadius: 'full',
          color: 'text.disabled',
          backgroundColor: 'surface.muted',
          display: isLastRowHovered && isLastColumnHovered ? 'flex' : 'none',
          opacity: '90',
          _hover: { display: 'flex', backgroundColor: 'interactive.hover' },
          _active: { color: 'text.bright', backgroundColor: 'accent.brand.default' },
        })}
        aria-label="행 및 열 추가"
        onclick={() => {
          insertAxis('horizontal', rowCount - 1, false);
          insertAxis('vertical', lastColumnIndex, false);
        }}
        type="button"
      >
        <span style:display="inline-flex" style:transform={fixedControlTransform} style:transform-origin="center center">
          <Icon icon={PlusIcon} size={14} />
        </span>
      </button>
    </div>
  {/if}

  <!-- Floating toolbar -->
  <div
    style:left="{overlay.bounds.x + overlay.bounds.width / 2}px"
    style:top="{overlay.bounds.y - floatingToolbarOffset}px"
    style:transform={fixedControlTransform}
    style:transform-origin="top center"
    class={center({
      position: 'absolute',
      width: 'auto',
      height: '30px',
      display: isActive ? 'flex' : 'none',
      gap: '2px',
      alignItems: 'center',
      translate: 'auto',
      translateX: '-1/2',
      pointerEvents: 'auto',
      zIndex: '50',
      backgroundColor: 'surface.default',
      borderRadius: '6px',
      boxShadow: 'small',
      borderWidth: '1px',
      borderColor: 'border.strong',
      padding: '2px',
      cursor: 'default',
    })}
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
              color: open ? 'text.default' : 'text.faint',
              backgroundColor: open ? 'interactive.hover' : 'transparent',
              width: '24px',
              height: '24px',
              borderRadius: '4px',
              cursor: 'pointer',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
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
              setAlign('left');
            }}
          >
            <Icon icon={AlignLeftIcon} size={14} />
            <span>왼쪽 정렬</span>
          </MenuItem>
          <MenuItem
            onclick={() => {
              close();
              setAlign('center');
            }}
          >
            <Icon icon={AlignCenterIcon} size={14} />
            <span>가운데 정렬</span>
          </MenuItem>
          <MenuItem
            onclick={() => {
              close();
              setAlign('right');
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
            color: open ? 'text.default' : 'text.faint',
            backgroundColor: open ? 'interactive.hover' : 'transparent',
            width: '24px',
            height: '24px',
            borderRadius: '4px',
            cursor: 'pointer',
            _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
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
            setBorderStyle('solid');
          }}
        >
          <div
            class={css({ width: '14px', height: '0', borderBottomWidth: '2px', borderBottomStyle: 'solid', borderColor: 'text.default' })}
          ></div>
          <span>실선</span>
        </MenuItem>
        <MenuItem
          onclick={() => {
            close();
            setBorderStyle('dashed');
          }}
        >
          <div
            class={css({ width: '14px', height: '0', borderBottomWidth: '2px', borderBottomStyle: 'dashed', borderColor: 'text.default' })}
          ></div>
          <span>파선</span>
        </MenuItem>
        <MenuItem
          onclick={() => {
            close();
            setBorderStyle('dotted');
          }}
        >
          <div
            class={css({ width: '14px', height: '0', borderBottomWidth: '2px', borderBottomStyle: 'dotted', borderColor: 'text.default' })}
          ></div>
          <span>점선</span>
        </MenuItem>
        <MenuItem
          onclick={() => {
            close();
            setBorderStyle('none');
          }}
        >
          <Icon icon={BanIcon} size={14} />
          <span>없음</span>
        </MenuItem>
      {/snippet}
    </Menu>
  </div>

  {#if cellSelectionButtonPosition !== null}
    <div
      style:left="{overlay.bounds.x + cellSelectionButtonPosition.left}px"
      style:top="{overlay.bounds.y + cellSelectionButtonPosition.top}px"
      class={center({
        position: 'absolute',
        translate: 'auto',
        translateX: '-1/2',
        zIndex: '50',
        pointerEvents: 'auto',
      })}
      onpointerdown={(e) => e.stopPropagation()}
      role="presentation"
    >
      <Menu offset={4} onopen={() => (cellBgMenuOpen = true)} ontransitionend={() => (cellBgMenuOpen = false)} placement="bottom">
        {#snippet button({ open })}
          <button
            class={center({
              display: 'flex',
              color: open ? 'text.default' : 'text.faint',
              backgroundColor: open ? 'interactive.hover' : 'surface.default',
              width: '26px',
              height: '26px',
              borderRadius: '6px',
              borderWidth: '1px',
              borderColor: 'border.strong',
              boxShadow: 'small',
              cursor: 'pointer',
              _hover: { backgroundColor: 'interactive.hover', color: 'text.default' },
            })}
            aria-pressed={open}
            type="button"
          >
            <Icon icon={PaintBucketIcon} size={14} />
          </button>
        {/snippet}
        {#snippet children({ close })}
          <li>
            <ToolbarColorGrid
              columns={8}
              currentValue={overlay.cell_selection?.background_color ?? 'none'}
              items={cellBackgroundColors}
              onClose={close}
              onSelect={(value) => {
                close();
                enqueueTableOp({
                  type: 'node',
                  op: {
                    type: 'table',
                    id: overlay.table_id,
                    op: {
                      type: 'set_cell_selection_background_color',
                      color: value === 'none' ? undefined : value,
                    },
                  },
                });
              }}
            />
          </li>
        {/snippet}
      </Menu>
    </div>
  {/if}
{/if}
