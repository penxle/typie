<script lang="ts">
  import { mergeAttributes } from '@tiptap/core';
  import { TableMap } from '@tiptap/pm/tables';
  import { tick } from 'svelte';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
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
</script>

<NodeView style={css.raw({ position: 'relative' })} {...HTMLAttributes}>
  <table
    style:--table-border-style={attrs.borderStyle}
    onpointerleave={() => {
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
        outlineColor: 'gray.300',
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
        {#each rowElems as row, i (row)}
          <div
            style:height={`${row.clientHeight}px`}
            style:top={`${row.offsetTop}px`}
            class={flex({
              position: 'absolute',
              left: '0',
              width: '18px',
              height: '24px',
              translateX: '-1/2',
              translate: 'auto',
              zIndex: '10',
              justifyContent: 'center',
              alignItems: 'center',
              pointerEvents: hoveredRowIndex === i ? 'auto' : 'none',
            })}
            role="row"
          >
            <RowHandle {editor} {getPos} {hasSpan} {hoveredRowIndex} {i} tableNode={node} />
          </div>
        {/each}
      </div>
      {#if colgroupRendered}
        {#each colElems as col, i (i)}
          <!-- svelte-ignore node_invalid_placement_ssr -->
          <div
            style:left={`${col.offsetLeft}px`}
            style:width={`${col.clientWidth}px`}
            class={flex({
              position: 'absolute',
              top: '0',
              width: '24px',
              height: '18px',
              translateY: '-1/2',
              translate: 'auto',
              zIndex: '10',
              justifyContent: 'center',
              alignItems: 'center',
              pointerEvents: hoveredColumnIndex === i ? 'auto' : 'none',
              '.block-selection-decoration &': {
                display: 'none',
              },
            })}
          >
            <ColHandle {editor} {getPos} {hasSpan} {hoveredColumnIndex} {i} tableNode={node} />
          </div>
        {/each}
      {/if}
    {/if}

    <NodeViewContentEditable
      style={css.raw({ '& p': { textIndent: '0!' }, '& td': { borderStyle: 'var(--table-border-style)' } })}
      as="tbody"
    />

    {#if editor?.current.isEditable}
      <AddRowColButton {editor} {getPos} {isLastColumnHovered} {isLastRowHovered} tableNode={node} />
    {/if}
  </table>
</NodeView>

<TiptapNodeViewBubbleMenu {editor} {getPos} {node}>
  <select onchange={(e) => updateAttributes({ borderStyle: e.currentTarget.value })}>
    <option selected={attrs.borderStyle === 'solid'} value="solid">solid</option>
    <option selected={attrs.borderStyle === 'dashed'} value="dashed">dashed</option>
    <option selected={attrs.borderStyle === 'dotted'} value="dotted">dotted</option>
    <option selected={attrs.borderStyle === 'none'} value="none">none</option>
  </select>
</TiptapNodeViewBubbleMenu>
