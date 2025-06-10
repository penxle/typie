<script lang="ts">
  import { CellSelection, TableMap } from '@tiptap/pm/tables';
  import ArrowLeftToLineIcon from '~icons/lucide/arrow-left-to-line';
  import ArrowRightToLineIcon from '~icons/lucide/arrow-right-to-line';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import MoveLeftIcon from '~icons/lucide/move-left';
  import MoveRightIcon from '~icons/lucide/move-right';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { Icon, Menu, MenuItem, Tooltip } from '$lib/components';
  import { center } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Node } from '@tiptap/pm/model';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    tableNode: Node;
    getPos: () => number | undefined;
    i: number;
    hoveredColumnIndex?: number | null;
    focusedColumnIndex?: number | null;
    hasSpan?: boolean;
  };

  let { editor, tableNode, getPos, i, hoveredColumnIndex, focusedColumnIndex, hasSpan = false }: Props = $props();

  const map = $derived(TableMap.get(tableNode));

  function selectColumn(colIndex: number) {
    if (!editor) {
      return;
    }

    const { tr } = editor.current.state;
    const tablePos = getPos();
    if (tablePos === undefined) {
      return;
    }

    const tableStart = tablePos + 1;

    if (colIndex < 0 || colIndex >= map.width) {
      return false;
    }

    const colCells = map.cellsInRect({
      left: colIndex,
      right: colIndex + 1,
      top: 0,
      bottom: map.height,
    });

    const anchorCell = tr.doc.resolve(tableStart + colCells[0]);
    // eslint-disable-next-line unicorn/prefer-at
    const headCell = tr.doc.resolve(tableStart + colCells[colCells.length - 1]);

    const colSelection = CellSelection.colSelection(anchorCell, headCell);
    editor.current.view.dispatch(tr.setSelection(colSelection));

    return true;
  }

  function swapColumns(a: number, b: number) {
    if (!editor) {
      return false;
    }

    const { tr } = editor.current.state;

    if (hasSpan) {
      return false;
    }

    let map = TableMap.get(tableNode);

    if (a < 0 || a >= map.width || b < 0 || b >= map.width) {
      return false;
    }

    if (a === b) {
      return false;
    }

    let rows = [];

    for (let rowIndex = 0; rowIndex < map.height; rowIndex++) {
      let row = tableNode.child(rowIndex);
      let cells = [];

      for (let cellIndex = 0; cellIndex < row.childCount; cellIndex++) {
        cells.push(row.child(cellIndex));
      }

      let temp = cells[a];
      cells[a] = cells[b];
      cells[b] = temp;

      let rowType = row.type;
      let newRow = rowType.createChecked(row.attrs, cells, row.marks);

      rows.push(newRow);
    }

    let tableType = tableNode.type;
    let newTable = tableType.createChecked(tableNode.attrs, rows, tableNode.marks);
    const tablePos = getPos();
    if (tablePos === undefined) {
      return;
    }

    tr.replaceWith(tablePos, tablePos + tableNode.nodeSize, newTable);

    editor.current.view.dispatch(tr);

    return true;
  }
</script>

<Menu
  offset={4}
  onopen={() => {
    if (!window.__webview__) {
      selectColumn(i);
    }
  }}
  placement="bottom-start"
>
  {#snippet button({ open })}
    <div
      class={center({
        display: open || hoveredColumnIndex === i || focusedColumnIndex === i ? 'flex' : 'none',
        _hover: {
          backgroundColor: 'gray.200',
        },
        _pressed: {
          color: 'white',
          backgroundColor: '[var(--prosemirror-color-selection)]',
          borderWidth: '0',
          _hover: {
            backgroundColor: '[var(--prosemirror-color-selection)]',
          },
        },
        width: '24px',
        height: '18px',
        color: 'gray.500',
        borderRadius: '4px',
        backgroundColor: 'white',
        borderWidth: '1px',
        borderColor: 'gray.300',
        boxShadow: 'medium',
      })}
      aria-pressed={open}
    >
      <Icon icon={EllipsisIcon} size={14} />
    </div>
  {/snippet}
  {#snippet children({ close })}
    {#if i !== 0}
      <Tooltip enabled={hasSpan} message="표에 병합된 셀이 없을 때만 이동할 수 있습니다.">
        <MenuItem
          disabled={hasSpan}
          onclick={() => {
            close();
            swapColumns(i, i - 1);
          }}
        >
          <Icon icon={MoveLeftIcon} size={14} />
          <span>왼쪽으로 이동</span>
        </MenuItem>
      </Tooltip>
    {/if}
    {#if i !== map.width - 1}
      <Tooltip enabled={hasSpan} message="표에 병합된 셀이 없을 때만 이동할 수 있습니다.">
        <MenuItem
          disabled={hasSpan}
          onclick={() => {
            close();
            swapColumns(i, i + 1);
          }}
        >
          <Icon icon={MoveRightIcon} size={14} />
          <span>오른쪽으로 이동</span>
        </MenuItem>
      </Tooltip>
    {/if}
    <MenuItem
      onclick={() => {
        close();
        editor?.current?.commands.addColumnBefore();
      }}
    >
      <Icon icon={ArrowLeftToLineIcon} size={14} />
      <span>왼쪽에 열 추가</span>
    </MenuItem>
    <MenuItem
      onclick={() => {
        close();
        editor?.current?.commands.addColumnAfter();
      }}
    >
      <Icon icon={ArrowRightToLineIcon} size={14} />
      <span>오른쪽에 열 추가</span>
    </MenuItem>
    <MenuItem
      onclick={() => {
        close();
        editor?.current?.commands.deleteColumn();
      }}
      variant="danger"
    >
      <Icon icon={Trash2Icon} size={14} />
      <span>열 삭제</span>
    </MenuItem>
    {#if window.__webview__}
      <MenuItem
        onclick={() => {
          close();
          editor?.current?.commands.deleteTable();
        }}
        variant="danger"
      >
        <Icon icon={Trash2Icon} size={14} />
        <span>표 삭제</span>
      </MenuItem>
    {/if}
  {/snippet}
</Menu>
