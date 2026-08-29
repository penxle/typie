<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon, Menu } from '@typie/ui/components';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import type { Snippet } from 'svelte';

  type Props = {
    label: string;
    children?: Snippet;
  };

  let { label, children }: Props = $props();
</script>

<Menu
  style={css.raw({
    display: 'none',
    flexShrink: '0',
    alignItems: 'center',
    justifyContent: 'center',
    borderRadius: '4px',
    size: '16px',
    color: 'text.disabled',
    cursor: 'pointer',
    transition: 'common',
    _hover: { backgroundColor: 'interactive.hover', color: 'text.subtle' },
    _focusVisible: { backgroundColor: 'interactive.hover', color: 'text.subtle' },
    _groupHover: { display: 'flex' },
    _groupFocusWithin: { display: 'flex' },
    _expanded: { display: 'flex', backgroundColor: 'interactive.hover', color: 'text.subtle' },
    '@media (pointer: coarse)': { display: 'flex' },
  })}
  buttonAriaLabel={label}
  placement="bottom-start"
>
  {#snippet button({ open })}
    <span class={css({ display: 'flex', alignItems: 'center', justifyContent: 'center', size: 'full' })} aria-pressed={open}>
      <Icon icon={EllipsisIcon} size={14} />
    </span>
  {/snippet}

  {@render children?.()}
</Menu>
