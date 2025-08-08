<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    title: string;
    description: string;
    icon: Component;
    children?: Snippet;
  };

  let { title, description, icon, children }: Props = $props();
</script>

<div class={flex({ direction: 'column', align: 'center' })}>
  <div class={center({ borderRadius: '8px', size: '40px', backgroundColor: 'surface.muted', color: 'text.subtle' })}>
    <Icon {icon} size={20} />
  </div>
  <p class={css({ marginTop: '10px', marginBottom: '2px', fontSize: '18px', fontWeight: 'semibold' })}>{title}</p>
  <p class={css({ marginBottom: '20px', fontSize: '14px', color: 'text.muted' })}>{description}</p>

  <div class={css({ width: 'full', maxWidth: '280px' })}>
    {@render children?.()}
  </div>
</div>
