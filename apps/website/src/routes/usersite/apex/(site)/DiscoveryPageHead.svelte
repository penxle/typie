<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    title: string;
    sub?: string;
  };

  let { title, sub }: Props = $props();
</script>

<div class={css({ marginBottom: '8px' })}>
  <h1 class={css({ fontSize: '22px', fontWeight: 'bold', letterSpacing: '-0.02em', lineHeight: '[1.3]' })}>{title}</h1>
  {#if sub}
    <div class={css({ marginTop: '6px', fontSize: '13px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>{sub}</div>
  {/if}
</div>
