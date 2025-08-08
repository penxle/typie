<script generics="ColumnKey extends `$${string}`, T extends Record<string, unknown>" lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Column = {
    key: ColumnKey;
    label: string;
    width?: string;
  };

  type Snippets = Record<ColumnKey, Snippet<[T]>>;

  type Props = {
    columns: Column[];
    dataKey: keyof T;
    data: T[] | undefined;
  } & Snippets;

  let { columns, dataKey, data, ...rest }: Props = $props();

  const snippets = $derived(rest as unknown as Snippets);
</script>

<div class={css({ overflowX: 'auto' })}>
  <table class={css({ width: 'full', borderCollapse: 'collapse', tableLayout: 'fixed' })}>
    <thead>
      <tr class={css({ borderBottomWidth: '2px', borderColor: 'amber.500' })}>
        {#each columns as column (column.key)}
          <th
            style={column.width ? `width: ${column.width}` : ''}
            class={css({
              paddingX: '20px',
              paddingY: '16px',
              fontSize: '11px',
              fontFamily: 'mono',
              fontWeight: 'normal',
              color: 'amber.500',
              textAlign: 'left',
            })}
          >
            {column.label}
          </th>
        {/each}
      </tr>
    </thead>
    <tbody>
      {#if data && data.length > 0}
        {#each data as item, i (item[dataKey])}
          <tr
            class={css({
              borderBottomWidth: i < data.length - 1 ? '1px' : '0',
              borderColor: 'gray.800',
              _hover: {
                backgroundColor: 'gray.800',
              },
            })}
          >
            {#each columns as column (column.key)}
              <td class={css({ padding: '20px', fontSize: '12px', color: 'amber.500' })}>
                {#if snippets[column.key]}
                  {@render snippets[column.key](item)}
                {:else}
                  {item[column.key]}
                {/if}
              </td>
            {/each}
          </tr>
        {/each}
      {:else}
        <tr>
          <td class={css({ padding: '64px', textAlign: 'center' })} colspan={columns.length}>
            <div class={css({ fontSize: '13px', fontFamily: 'mono', color: 'amber.400' })}>NO DATA FOUND</div>
          </td>
        </tr>
      {/if}
    </tbody>
  </table>
</div>
