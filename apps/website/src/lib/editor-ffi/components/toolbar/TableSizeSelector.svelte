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
    display: 'grid',
    gridTemplateColumns: 'repeat(10, minmax(0, 1fr))',
    gap: '2px',
    padding: '8px',
  })}
  onpointerleave={() => {
    hoveredRow = 0;
    hoveredCol = 0;
  }}
  role="presentation"
>
  {#each { length: maxRows }, rowIdx (rowIdx)}
    {#each { length: maxCols }, colIdx (colIdx)}
      <button
        class={css({
          aspectRatio: '[1 / 1]',
          borderWidth: '1px',
          borderStyle: 'solid',
          borderColor: rowIdx < hoveredRow && colIdx < hoveredCol ? 'accent.default' : 'border.default',
          borderRadius: '2px',
          backgroundColor: rowIdx < hoveredRow && colIdx < hoveredCol ? 'surface.active' : 'surface.default',
          transition: '[background-color 0.1s ease]',
          cursor: 'pointer',
          _hover: {
            borderColor: 'accent.default',
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
