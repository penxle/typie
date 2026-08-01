<script generics="ColumnKey extends `$${string}`, T extends Record<string, unknown>" lang="ts">
  import { css } from '@typie/styled-system/css';
  import AdminEmpty from './AdminEmpty.svelte';
  import type { Snippet } from 'svelte';

  type Column = {
    key: ColumnKey;
    label: string;
    width?: string;
  };

  type Snippets = Record<ColumnKey, Snippet<[T]>>;

  type Props = Snippets & {
    columns: Column[];
    dataKey: keyof T;
    data: T[] | undefined;
    emptyText?: string;
    filters?: Snippet;
    footer?: Snippet;
  };

  let { columns, dataKey, data, emptyText, filters, footer, ...rest }: Props = $props();

  const snippets = $derived(rest as unknown as Snippets);
</script>

<div
  class={css({
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '12px',
    backgroundColor: 'admin.card.default',
    boxShadow: 'adminCard',
    overflow: 'hidden',
  })}
>
  {#if filters}
    <div class={css({ paddingX: '14px', paddingY: '10px' })}>
      {@render filters()}
    </div>
  {/if}

  <div class={css({ overflowX: 'auto' })}>
    <table class={css({ width: 'full', borderCollapse: 'collapse', tableLayout: 'fixed' })}>
      <thead>
        <tr
          class={css({
            borderTopWidth: filters ? '1px' : '0',
            borderBottomWidth: '1px',
            borderColor: 'border.subtle',
            backgroundColor: 'admin.card.hover',
          })}
        >
          {#each columns as column (column.key)}
            <th
              style={column.width ? `width: ${column.width}` : ''}
              class={css({
                paddingX: '14px',
                paddingY: '8px',
                fontSize: '11px',
                fontWeight: 'semibold',
                letterSpacing: '[0.05em]',
                color: 'text.faint',
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
                borderColor: 'border.subtle',
                _hover: { backgroundColor: 'admin.card.hover' },
              })}
            >
              {#each columns as column (column.key)}
                <td
                  class={css({
                    paddingX: '14px',
                    paddingY: '12px',
                    fontSize: '13px',
                    color: 'text.default',
                    fontVariantNumeric: 'tabular-nums',
                  })}
                >
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
            <td colspan={columns.length}>
              <AdminEmpty text={emptyText} />
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if footer}
    <div class={css({ borderTopWidth: '1px', borderColor: 'border.subtle', paddingX: '14px', paddingY: '10px' })}>
      {@render footer()}
    </div>
  {/if}
</div>
