<script generics="T" lang="ts">
  import { css } from '@typie/styled-system/css';
  import { tooltip } from '@typie/ui/actions';
  import { labelGlossOf } from '$lib/feedback/glosses.ts';
  import type { Snippet } from 'svelte';
  import type { LabelField } from '$lib/feedback/glosses.ts';

  // 열 머리글은 원문 키다(field가 있으면 한국어 풀이 툴팁). 행 스니펫은 <td>들을 그린다. 바깥 프레임 없이 헤어라인 행만 —
  // 첫 열은 본문 왼끝에 맞춘다. rowId는 교차 참조의 착지 자리(data-flash 강조는 항목과 같다).
  type Column = { label: string; field?: LabelField };
  type Props = { columns: Column[]; rows: T[]; cell: Snippet<[T, number]>; rowId?: (row: T, index: number) => string | undefined };
  const { columns, rows, cell, rowId }: Props = $props();
</script>

<div class={css({ width: 'full', overflowX: 'auto' })}>
  <table
    class={css({
      width: 'full',
      borderCollapse: 'collapse',
      fontSize: '13px',
      lineHeight: '[1.65]',
      '& th, & td': { paddingX: '10px', textAlign: 'left', verticalAlign: 'top' },
      '& th:first-child, & td:first-child': { paddingLeft: '0' },
      '& th:last-child, & td:last-child': { paddingRight: '0' },
      '& th': {
        paddingY: '6px',
        borderBottomWidth: '1px',
        borderColor: 'border.default',
        fontFamily: 'mono',
        fontSize: '11px',
        letterSpacing: '0',
        fontWeight: 'medium',
        color: 'text.faint',
        whiteSpace: 'nowrap',
      },
      '& th > span[data-gloss]': { cursor: 'help' },
      '& td': {
        paddingY: '10px',
        borderBottomWidth: '1px',
        borderColor: 'border.subtle',
        color: 'text.default',
        wordBreak: 'keep-all',
        overflowWrap: 'anywhere',
      },
      '& tbody tr:last-child td': { borderBottomWidth: '0' },
      '& tbody tr': { transition: 'colors', transitionDuration: '[250ms]' },
      '& tbody tr[data-flash]': { backgroundColor: 'accent.brand.subtle' },
    })}
  >
    <thead>
      <tr>
        {#each columns as column (column.label)}
          <th>
            {#if column.field}
              <span data-gloss use:tooltip={{ message: labelGlossOf(column.field), placement: 'top', delay: 200 }}>{column.label}</span>
            {:else}
              {column.label}
            {/if}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#each rows as row, index (index)}
        <tr id={rowId?.(row, index)}>
          {@render cell(row, index)}
        </tr>
      {/each}
    </tbody>
  </table>
</div>
