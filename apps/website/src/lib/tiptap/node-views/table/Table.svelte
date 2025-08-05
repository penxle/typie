<script lang="ts">
  import { hide } from '@floating-ui/dom';
  import { mergeAttributes } from '@tiptap/core';
  import { TableMap } from '@tiptap/pm/tables';
  import { onMount, tick } from 'svelte';
  import { createFloatingActions, portal } from '$lib/actions';
  import { css } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import { TiptapNodeViewBubbleMenu } from '../../components';
  import { NodeView, NodeViewContentEditable } from '../../lib';
  import AddRowColButton from './AddRowColButton.svelte';
  import ColHandle from './ColHandle.svelte';
  import RowHandle from './RowHandle.svelte';
  import { createColGroup } from './utils';
  import type { Node } from '@tiptap/pm/model';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let colgroupRendered = $state(false);

  // eslint-disable-next-line unicorn/prefer-top-level-await
  tick().then(() => {
    colgroupRendered = true;
  });

  let { node, HTMLAttributes, editor, getPos, updateAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const hasSpan = $derived.by(() => {
    let has = false;

    node.descendants((node) => {
      if (node.type.name === 'tableCell' && (attrs.colspan > 1 || attrs.rowspan > 1)) {
        has = true;
        return;
      }
    });

    return has;
  });

  const { colgroup, tableWidth, tableMinWidth } = $derived(createColGroup(node, 50));

  // @ts-expect-error colgroup type mismatch
  const cols = $derived((colgroup?.slice(2) as ['col', Record<string, string>][]) ?? []);

  let _colElems = $state<HTMLElement[]>([]);
  const colElems = $derived(_colElems.filter(Boolean)); // 열 삭제에 대응

  let rowElems = $state<HTMLElement[]>([]);
  let tableElement = $state<HTMLTableElement>();

  async function getRows(tableNode: Node) {
    if (!editor || !tableNode) {
      return;
    }

    const { state, view } = editor.current;

    const map = TableMap.get(tableNode);
    const rowsLength = map.height;
    const tablePos = getPos();
    if (tablePos === undefined) {
      return;
    }

    const tableStart = tablePos + 1;

    // table row가 렌더링되길 기다림
    await tick();

    rowElems = [];
    for (let i = 0; i < rowsLength; i++) {
      const pos = map.positionAt(i, 0, tableNode);
      const cellPos = tableStart + pos;
      const rowPos = state.doc.resolve(cellPos - 1);
      const row = view.nodeDOM(rowPos.pos);
      if (row) {
        rowElems.push(row as HTMLElement);
      }
    }
  }

  $effect(() => {
    getRows(node);
  });

  let hoveredRowIndex = $state<number | null>(null);
  let hoveredColumnIndex = $state<number | null>(null);
  let focusedRowIndex = $state<number | null>(null);
  let focusedColumnIndex = $state<number | null>(null);
  const isLastRowHovered = $derived(hoveredRowIndex === rowElems.length - 1);
  const isLastColumnHovered = $derived(hoveredColumnIndex === cols.length - 1);

  function handlePointerover(event: PointerEvent) {
    const target = event.target as HTMLElement;

    const cell = target.closest('td,th');

    if (cell) {
      hoveredColumnIndex = (cell as HTMLTableCellElement).cellIndex;
      hoveredRowIndex = (cell.parentElement as HTMLTableRowElement).rowIndex;

      const prevCols = [...(cell.parentElement?.children ?? [])].slice(0, hoveredColumnIndex);
      // 왼쪽에 병합된 열이 있는 경우를 고려한 hoveredColumnIndex
      hoveredColumnIndex = prevCols.reduce((acc, col) => acc + (col as HTMLTableCellElement).colSpan, 0);
    }
  }

  function getSelectedCellPosition() {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return null;

    const range = selection.getRangeAt(0);
    let node = range.startContainer as HTMLElement | null;

    while (node && node.nodeName.toLowerCase() !== 'td') {
      node = node.parentElement;
    }

    if (!node || node.nodeName.toLowerCase() !== 'td') return null;

    const td = node;
    const tr = td.parentElement;
    const table = tr?.parentElement;

    if (!tr || !table) return null;

    const rowIndex = [...table.children].indexOf(tr);
    const colIndex = [...tr.children].indexOf(td);

    return { rowIndex, colIndex };
  }

  function handleSelectionChange() {
    const pos = getSelectedCellPosition();

    if (pos) {
      focusedRowIndex = pos.rowIndex;
      focusedColumnIndex = pos.colIndex;
    } else {
      focusedRowIndex = null;
      focusedColumnIndex = null;
    }
  }

  let tableSize = $state({ width: 0, height: 0 });

  const rowPositions = $derived.by(() => {
    if (rowElems.length === 0 || !tableElement) return [];

    void tableSize.height;

    return rowElems.map((row) => ({
      top: row.offsetTop,
      height: row.clientHeight,
    }));
  });

  const colPositions = $derived.by(() => {
    if (colElems.length === 0 || !tableElement) return [];

    void tableSize.width;

    return colElems.map((col) => ({
      left: col.offsetLeft,
      width: col.offsetWidth,
    }));
  });

  $effect(() => {
    if (tableElement) {
      const resizeObserver = new ResizeObserver(([entry]) => {
        tableSize = {
          width: entry.contentRect.width,
          height: entry.contentRect.height,
        };
      });

      resizeObserver.observe(tableElement);

      return () => {
        resizeObserver.disconnect();
      };
    }
  });

  onMount(() => {
    if (window.__webview__) {
      document.addEventListener('selectionchange', handleSelectionChange);
      return () => document.removeEventListener('selectionchange', handleSelectionChange);
    }
  });
</script>

<NodeView style={css.raw({ position: 'relative' })} {...HTMLAttributes}>
  <div
    class={css({
      overflowX: 'auto',
      overflowY: 'hidden',
    })}
  >
    <table
      bind:this={tableElement}
      style:--table-border-style={attrs.borderStyle}
      onpointerleave={(e) => {
        const relatedTarget = e.relatedTarget as HTMLElement | null;
        if (relatedTarget?.closest('[data-floating-row-handle], [data-floating-col-handle]')) {
          return;
        }

        hoveredRowIndex = null;
        hoveredColumnIndex = null;
      }}
      onpointerover={handlePointerover}
      {...mergeAttributes(HTMLAttributes, {
        class: css({
          position: 'relative',
          borderRadius: '4px',
          borderStyle: 'hidden',
          outlineWidth: '1px',
          outlineStyle: 'var(--table-border-style)',
          outlineOffset: '-1px',
          outlineColor: 'border.strong',
        }),
        style: tableWidth ? `width: ${tableWidth}` : `min-width: ${tableMinWidth}`,
      })}
    >
      <colgroup>
        {#each cols as col, i (col)}
          <col bind:this={_colElems[i]} {...col[1]} />
        {/each}
      </colgroup>
      {#if editor?.current.isEditable}
        <!-- svelte-ignore node_invalid_placement_ssr -->
        <div
          class={css({
            position: 'absolute',
            inset: '0',
          })}
          contenteditable={false}
          role="rowgroup"
        >
          {#each rowPositions as pos, i (i)}
            {@const { anchor, floating } = createFloatingActions({
              placement: 'left',
              offset: -9,
              middleware: [hide()],
            })}
            <div
              style:height={`${pos.height}px`}
              style:top={`${pos.top}px`}
              class={center({
                position: 'absolute',
                left: '0',
                translate: 'auto',
                width: '18px',
                height: '24px',
                pointerEvents: hoveredRowIndex === i || focusedRowIndex === i ? 'auto' : 'none',
              })}
              role="row"
              use:anchor
            >
              <div class={css({ zIndex: '10' })} data-floating-row-handle use:floating use:portal>
                <RowHandle {editor} {focusedRowIndex} {getPos} {hasSpan} {hoveredRowIndex} {i} tableNode={node} />
              </div>
            </div>
          {/each}
        </div>
        {#if colgroupRendered && colPositions.length > 0}
          <!-- svelte-ignore node_invalid_placement_ssr -->
          {#each colPositions as pos, i (i)}
            {@const { anchor, floating } = createFloatingActions({
              placement: 'top',
              offset: -12,
              middleware: [hide()],
            })}
            <div
              style:left={`${pos.left}px`}
              style:width={`${pos.width}px`}
              class={center({
                position: 'absolute',
                top: '0',
                translate: 'auto',
                width: '24px',
                height: '18px',
                pointerEvents: hoveredColumnIndex === i || focusedColumnIndex === i ? 'auto' : 'none',
                '.block-selection-decoration &': {
                  display: 'none',
                },
              })}
              contenteditable={false}
              use:anchor
            >
              <div class={css({ zIndex: '10' })} data-floating-col-handle use:floating use:portal>
                <ColHandle {editor} {focusedColumnIndex} {getPos} {hasSpan} {hoveredColumnIndex} {i} tableNode={node} />
              </div>
            </div>
          {/each}
        {/if}
      {/if}

      <NodeViewContentEditable
        style={css.raw({ '& p': { textIndent: '0!' }, '& td': { borderStyle: 'var(--table-border-style)' } })}
        as="tbody"
      />
    </table>
    {#if editor?.current.isEditable && !window.__webview__}
      <AddRowColButton {editor} {getPos} {isLastColumnHovered} {isLastRowHovered} tableNode={node} />
    {/if}
  </div>
</NodeView>

<TiptapNodeViewBubbleMenu {editor} {getPos} {node}>
  <select class={css({ cursor: 'pointer' })} onchange={(e) => updateAttributes({ borderStyle: e.currentTarget.value })}>
    <option selected={attrs.borderStyle === 'solid'} value="solid">solid</option>
    <option selected={attrs.borderStyle === 'dashed'} value="dashed">dashed</option>
    <option selected={attrs.borderStyle === 'dotted'} value="dotted">dotted</option>
    <option selected={attrs.borderStyle === 'none'} value="none">none</option>
  </select>
</TiptapNodeViewBubbleMenu>
