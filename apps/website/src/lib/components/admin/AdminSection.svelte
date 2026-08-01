<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = {
    title: string;
    children: Snippet;
  };

  let { title, children }: Props = $props();
</script>

<section>
  <h2 class={css({ marginBottom: '10px', fontSize: '11px', fontWeight: 'semibold', letterSpacing: '[0.05em]', color: 'text.faint' })}>
    {title}
  </h2>
  <div
    class={css({
      borderWidth: '1px',
      borderColor: 'border.subtle',
      borderRadius: '12px',
      backgroundColor: 'admin.card.default',
      boxShadow: 'adminCard',
      padding: '16px',
    })}
  >
    {@render children()}
  </div>
</section>
