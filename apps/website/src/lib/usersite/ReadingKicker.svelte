<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Props = {
    items: readonly { label: string; href: string }[];
    style?: SystemStyleObject;
  };

  let { items, style }: Props = $props();
</script>

<nav
  class={css(
    {
      display: 'flex',
      alignItems: 'center',
      gap: '2px',
      width: 'fit',
      maxWidth: 'full',
      minWidth: '0',
      fontSize: '13px',
      fontWeight: 'medium',
      color: 'text.muted',
    },
    style,
  )}
>
  {#each items as item, index (item.href)}
    {#if index > 0}
      <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={ChevronRightIcon} size={14} />
    {/if}
    <a class={css({ minWidth: '0', truncate: true, transition: 'colors', _hover: { color: 'text.default' } })} href={item.href}>
      {item.label}
    </a>
  {/each}
</nav>
