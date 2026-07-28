<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Item = {
    label: string;
    value?: string | null;
    content?: Snippet;
    mono?: boolean;
  };

  type Props = {
    items: Item[];
  };

  let { items }: Props = $props();
</script>

<dl class={css({ display: 'grid', gridTemplateColumns: '[160px 1fr]', rowGap: '10px', columnGap: '16px', fontSize: '13px' })}>
  {#each items as item (item.label)}
    <dt class={css({ color: 'text.faint' })}>{item.label}</dt>
    <dd class={css({ color: 'text.default', wordBreak: 'break-all' }, item.mono && { fontFamily: 'mono' })}>
      {#if item.content}
        {@render item.content()}
      {:else if item.value === null || item.value === undefined || item.value === ''}
        <span class={css({ color: 'text.disabled' })}>—</span>
      {:else}
        {item.value}
      {/if}
    </dd>
  {/each}
</dl>
