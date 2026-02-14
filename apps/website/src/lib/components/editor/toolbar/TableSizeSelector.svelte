<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    onSelect: (rows: number, cols: number) => void;
  };

  let { onSelect }: Props = $props();

  const maxRows = 10;
  const maxCols = 10;

  let hoveredRow = $state(0);
  let hoveredCol = $state(0);
</script>

<div
  class={css({
    padding: '8px',
    display: 'flex',
    flexDirection: 'column',
    alignItems: 'center',
    gap: '8px',
  })}
  onpointerleave={() => {
    hoveredRow = 0;
    hoveredCol = 0;
  }}
>
  <div
    class={css({
      display: 'grid',
      gridTemplateColumns: `repeat(${maxCols}, 1fr)`,
      gap: '2px',
    })}
  >
    {#each { length: maxRows }, rowIdx (rowIdx)}
      {#each { length: maxCols }, colIdx (colIdx)}
        <button
          class={css({
            width: '18px',
            height: '18px',
            borderWidth: '1px',
            borderStyle: 'solid',
            borderColor: rowIdx < hoveredRow && colIdx < hoveredCol ? 'accent.brand.default' : 'border.default',
            borderRadius: '2px',
            backgroundColor: rowIdx < hoveredRow && colIdx < hoveredCol ? 'accent.brand.default/25' : 'surface.default',
            transition: '[background-color 0.1s ease]',
            cursor: 'pointer',
            _hover: {
              borderColor: 'accent.brand.default',
            },
          })}
          aria-label={`${rowIdx + 1} x ${colIdx + 1}`}
          onclick={() => {
            onSelect(rowIdx + 1, colIdx + 1);
          }}
          onpointerenter={() => {
            hoveredRow = rowIdx + 1;
            hoveredCol = colIdx + 1;
          }}
          type="button"
        ></button>
      {/each}
    {/each}
  </div>

  <div
    class={css({
      fontSize: '12px',
      color: 'text.subtle',
      fontVariantNumeric: 'tabular-nums',
    })}
  >
    {#if hoveredRow > 0 && hoveredCol > 0}
      {hoveredRow} × {hoveredCol}
    {:else}
      표 크기 선택
    {/if}
  </div>
</div>
