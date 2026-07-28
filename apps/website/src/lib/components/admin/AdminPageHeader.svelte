<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import type { Snippet } from 'svelte';

  type Props = {
    title: string;
    description?: string;
    badges?: Snippet;
    actions?: Snippet;
  };

  let { title, description, badges, actions }: Props = $props();
</script>

<div class={flex({ alignItems: 'flex-start', justifyContent: 'space-between', gap: '16px', marginBottom: '24px' })}>
  <div>
    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      <h1 class={css({ fontSize: '20px', fontWeight: 'extrabold', letterSpacing: '[-0.01em]', color: 'text.default' })}>{title}</h1>
      {#if badges}
        <div class={flex({ alignItems: 'center', gap: '6px' })}>
          {@render badges()}
        </div>
      {/if}
    </div>
    {#if description}
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>{description}</p>
    {/if}
  </div>
  {#if actions}
    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      {@render actions()}
    </div>
  {/if}
</div>
