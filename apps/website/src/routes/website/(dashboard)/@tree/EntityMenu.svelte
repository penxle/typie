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
  let menuOpen = $state(false);
  let menuPresented = $state(false);

  $effect(() => {
    if (menuOpen) menuPresented = true;
  });
</script>

<Menu
  style={css.raw(
    {
      display: 'none',
      flexShrink: '0',
      alignItems: 'center',
      justifyContent: 'center',
      borderRadius: '4px',
      size: '16px',
      color: 'text.muted',
      cursor: 'pointer',
      transition: 'common',
      _hover: { backgroundColor: 'surface.hover', color: 'text.default' },
      _focusVisible: { backgroundColor: 'surface.hover', color: 'text.default' },
      _groupHover: { display: 'flex' },
      _groupFocusWithin: { display: 'flex' },
      _expanded: { display: 'flex', backgroundColor: 'surface.active', color: 'text.default' },
      '@media (pointer: coarse)': { display: 'flex' },
    },
    menuPresented && { display: 'flex' },
  )}
  buttonAriaLabel={label}
  ontransitionend={() => (menuPresented = false)}
  placement="bottom-start"
  bind:open={menuOpen}
>
  {#snippet button({ open })}
    <span class={css({ display: 'flex', alignItems: 'center', justifyContent: 'center', size: 'full' })} aria-pressed={open}>
      <Icon icon={EllipsisIcon} size={14} />
    </span>
  {/snippet}

  {@render children?.()}
</Menu>
