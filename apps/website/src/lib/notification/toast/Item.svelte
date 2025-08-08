<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import Component from '../sonner/Component.svelte';

  type Props = {
    message: string;
    description?: string;
  };

  const { message, description, ...rest }: Props = $props();
</script>

<Component {...rest}>
  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>
      {message}
    </div>

    {#if description}
      <div class={css({ fontSize: '13px', color: 'text.faint' })}>{description}</div>
    {/if}
  </div>
</Component>
