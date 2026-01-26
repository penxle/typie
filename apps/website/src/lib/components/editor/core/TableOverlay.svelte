<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import { clamp } from '@typie/ui/utils';
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

  let hoveredColIndex = $state<number | null>(null);
  let hoveredRowIndex = $state<number | null>(null);

  // Track which menu is open (keeps handle visible during menu interaction)
  let menuOpenColIndex = $state<number | null>(null);
  let menuOpenRowIndex = $state<number | null>(null);

  // Separate state for add button hover (don't trigger handles)
  let addColButtonHovered = $state(false);
  let addRowButtonHovered = $state(false);
  let addBothButtonHovered = $state(false);

  const isLastRowHovered = $derived(hoveredRowIndex === overlay.rowHeights.length - 1 || addRowButtonHovered || addBothButtonHovered);
  const isLastColumnHovered = $derived(hoveredColIndex === overlay.colWidths.length - 1 || addColButtonHovered || addBothButtonHovered);

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

  function getColLeft(colIndex: number): number {
    if (colIndex === 0) {
      return 0;
    }
    return overlay.colPositions[colIndex - 1];
  }

  function getColWidth(colIndex: number): number {
    return overlay.colWidths[colIndex];
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

  function handleCellHover(rowIndex: number, colIndex: number) {
    hoveredRowIndex = rowIndex;
    hoveredColIndex = colIndex;
  }

  function handleCellLeave() {
    hoveredRowIndex = null;
    hoveredColIndex = null;
  }

  let isTableHovered = $state(false);
  let menuOpen = $state(false);
  let buttonHovered = $state(false);

  const isButtonVisible = $derived(isTableHovered || overlay.isFocused || menuOpen || buttonHovered);
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
  onpointerenter={() => (isTableHovered = true)}
  onpointerleave={() => (isTableHovered = false)}
>
  {#each overlay.rowHeights, rowIndex (rowIndex)}
    {#each overlay.colWidths, colIndex (colIndex)}
      {@const left = getColLeft(colIndex)}
      {@const top = getRowTop(rowIndex)}
      {@const width = getColWidth(colIndex)}
      {@const height = getRowHeight(rowIndex)}
      <div
        style:left="{left}px"
        style:top="{top}px"
        style:width="{width}px"
        style:height="{height}px"
        class={css({
          position: 'absolute',
          pointerEvents: 'auto',
        })}
        onpointerenter={() => handleCellHover(rowIndex, colIndex)}
        onpointerleave={handleCellLeave}
      ></div>
    {/each}
  {/each}

  {#each overlay.colWidths as width, i (i)}
    {@const left = getColLeft(i)}
    {@const isVisible = hoveredColIndex === i || menuOpenColIndex === i}
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
      })}
      onpointerenter={() => (hoveredColIndex = i)}
      onpointerleave={() => (hoveredColIndex = null)}
    >
      <Menu
        offset={4}
        onopen={() => {
          menuOpenColIndex = i;
          editor.dispatch({ type: 'selectTableColumn', tableId: overlay.tableId, col: i });
        }}
        ontransitionend={() => {
          menuOpenColIndex = null;
        }}
        placement="bottom-start"
      >
        {#snippet button({ open })}
          <button
            class={center({
              display: open || isVisible ? 'flex' : 'none',
              width: '24px',
              height: '18px',
              backgroundColor: 'surface.default',
              borderWidth: '1px',
              borderColor: 'border.strong',
              borderRadius: '4px',
              color: 'text.faint',
              boxShadow: 'medium',
              cursor: 'pointer',
              _hover: {
                backgroundColor: 'interactive.hover',
              },
              _pressed: {
                color: 'text.bright',
                backgroundColor: 'accent.brand.default',
                borderWidth: '0',
              },
            })}
            aria-pressed={open}
            type="button"
          >
            <Icon icon={EllipsisIcon} size={14} />
          </button>
        {/snippet}

        {#snippet children({ close })}
          {#if i > 0}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'moveTableColumn', tableId: overlay.tableId, fromCol: i, toCol: i - 1 });
                editor.focus();
              }}
            >
              <Icon icon={MoveLeftIcon} size={14} />
              <span>왼쪽으로 이동</span>
            </MenuItem>
          {/if}
          {#if i < overlay.colWidths.length - 1}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'moveTableColumn', tableId: overlay.tableId, fromCol: i, toCol: i + 1 });
                editor.focus();
              }}
            >
              <Icon icon={MoveRightIcon} size={14} />
              <span>오른쪽으로 이동</span>
            </MenuItem>
          {/if}
          {#if i > 0}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, afterCol: i - 1 });
                editor.focus();
              }}
            >
              <Icon icon={ArrowLeftToLineIcon} size={14} />
              <span>왼쪽에 열 추가</span>
            </MenuItem>
          {/if}
          <MenuItem
            onclick={() => {
              close();
              editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, afterCol: i });
              editor.focus();
            }}
          >
            <Icon icon={ArrowRightToLineIcon} size={14} />
            <span>오른쪽에 열 추가</span>
          </MenuItem>
          <MenuItem
            onclick={() => {
              close();
              editor.dispatch({ type: 'deleteTableColumn', tableId: overlay.tableId, col: i });
              editor.focus();
            }}
            variant="danger"
          >
            <Icon icon={Trash2Icon} size={14} />
            <span>열 삭제</span>
          </MenuItem>
        {/snippet}
      </Menu>
    </div>
  {/each}

  {#each overlay.rowHeights as height, i (i)}
    {@const top = getRowTop(i)}
    {@const isVisible = hoveredRowIndex === i || menuOpenRowIndex === i}
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
      onpointerenter={() => (hoveredRowIndex = i)}
      onpointerleave={() => (hoveredRowIndex = null)}
    >
      <Menu
        offset={4}
        onopen={() => {
          menuOpenRowIndex = i;
          editor.dispatch({ type: 'selectTableRow', tableId: overlay.tableId, row: i });
        }}
        ontransitionend={() => {
          menuOpenRowIndex = null;
        }}
        placement="right-start"
      >
        {#snippet button({ open })}
          <button
            class={center({
              display: open || isVisible ? 'flex' : 'none',
              width: '18px',
              height: '24px',
              backgroundColor: 'surface.default',
              borderWidth: '1px',
              borderColor: 'border.strong',
              borderRadius: '4px',
              color: 'text.faint',
              boxShadow: 'medium',
              cursor: 'pointer',
              _hover: {
                backgroundColor: 'interactive.hover',
              },
              _pressed: {
                color: 'text.bright',
                backgroundColor: 'accent.brand.default',
                borderWidth: '0',
              },
            })}
            aria-pressed={open}
            type="button"
          >
            <Icon icon={EllipsisVerticalIcon} size={14} />
          </button>
        {/snippet}

        {#snippet children({ close })}
          {#if i > 0}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'moveTableRow', tableId: overlay.tableId, fromRow: i, toRow: i - 1 });
                editor.focus();
              }}
            >
              <Icon icon={MoveUpIcon} size={14} />
              <span>위로 이동</span>
            </MenuItem>
          {/if}
          {#if i < overlay.rowHeights.length - 1}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'moveTableRow', tableId: overlay.tableId, fromRow: i, toRow: i + 1 });
                editor.focus();
              }}
            >
              <Icon icon={MoveDownIcon} size={14} />
              <span>아래로 이동</span>
            </MenuItem>
          {/if}
          {#if i > 0}
            <MenuItem
              onclick={() => {
                close();
                editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, afterRow: i - 1 });
                editor.focus();
              }}
            >
              <Icon icon={ArrowUpToLineIcon} size={14} />
              <span>위에 행 추가</span>
            </MenuItem>
          {/if}
          <MenuItem
            onclick={() => {
              close();
              editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, afterRow: i });
              editor.focus();
            }}
          >
            <Icon icon={ArrowDownToLineIcon} size={14} />
            <span>아래에 행 추가</span>
          </MenuItem>
          <MenuItem
            onclick={() => {
              close();
              editor.dispatch({ type: 'deleteTableRow', tableId: overlay.tableId, row: i });
              editor.focus();
            }}
            variant="danger"
          >
            <Icon icon={Trash2Icon} size={14} />
            <span>행 삭제</span>
          </MenuItem>
        {/snippet}
      </Menu>
    </div>
  {/each}

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
  style:left="{overlay.bounds.x + overlay.bounds.width}px"
  style:top="{overlay.bounds.y}px"
  style:height="{overlay.bounds.height}px"
  class={css({
    position: 'absolute',
    width: '23px',
    translate: 'auto',
    paddingLeft: '5px',
    pointerEvents: 'auto',
  })}
  data-external-element
  onpointerenter={() => (addColButtonHovered = true)}
  onpointerleave={() => (addColButtonHovered = false)}
>
  <button
    class={center({
      width: '18px',
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
      editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, afterCol: overlay.colWidths.length - 1 });
      editor.focus();
    }}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>

<div
  style:left="{overlay.bounds.x}px"
  style:top="{overlay.bounds.y + overlay.bounds.height}px"
  style:width="{overlay.bounds.width}px"
  class={css({
    position: 'absolute',
    height: '23px',
    translate: 'auto',
    paddingTop: '5px',
    pointerEvents: 'auto',
  })}
  data-external-element
  onpointerenter={() => (addRowButtonHovered = true)}
  onpointerleave={() => (addRowButtonHovered = false)}
>
  <button
    class={center({
      width: 'full',
      height: '18px',
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
      editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, afterRow: overlay.rowHeights.length - 1 });
      editor.focus();
    }}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>

<div
  style:left="{overlay.bounds.x + overlay.bounds.width}px"
  style:top="{overlay.bounds.y + overlay.bounds.height}px"
  class={css({
    position: 'absolute',
    width: '23px',
    height: '23px',
    translate: 'auto',
    paddingLeft: '5px',
    paddingTop: '5px',
    pointerEvents: 'auto',
  })}
  data-external-element
  onpointerenter={() => (addBothButtonHovered = true)}
  onpointerleave={() => (addBothButtonHovered = false)}
>
  <button
    class={center({
      width: '18px',
      height: '18px',
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
      editor.dispatch({ type: 'addTableRow', tableId: overlay.tableId, afterRow: overlay.rowHeights.length - 1 });
      editor.dispatch({ type: 'addTableColumn', tableId: overlay.tableId, afterCol: overlay.colWidths.length - 1 });
      editor.focus();
    }}
    type="button"
  >
    <Icon icon={PlusIcon} size={14} />
  </button>
</div>

<div
  style:left="{overlay.bounds.x + overlay.bounds.width / 2}px"
  style:top="{overlay.bounds.y - 38}px"
  class={center({
    position: 'absolute',
    width: '32px',
    height: '32px',
    translate: 'auto',
    translateX: '-1/2',
    pointerEvents: 'auto',
    zIndex: '1',
  })}
  data-external-element
  onpointerenter={() => (buttonHovered = true)}
  onpointerleave={() => (buttonHovered = false)}
>
  <Menu offset={4} onopen={() => (menuOpen = true)} ontransitionend={() => (menuOpen = false)} placement="bottom">
    {#snippet button({ open })}
      <button
        class={center({
          display: isButtonVisible ? 'flex' : 'none',
          fontSize: '14px',
          fontWeight: 'medium',
          color: 'text.faint',
          backgroundColor: 'surface.default',
          width: '24px',
          height: '24px',
          borderRadius: '4px',
          boxShadow: 'medium',
          borderWidth: '1px',
          borderColor: 'border.strong',
          cursor: 'pointer',
          _hover: {
            backgroundColor: 'interactive.hover',
          },
          _pressed: {
            color: 'text.bright',
            backgroundColor: 'accent.brand.default',
            borderWidth: '0',
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
