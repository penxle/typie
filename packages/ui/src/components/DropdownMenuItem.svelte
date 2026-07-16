<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { Icon } from '@typie/ui/components';
  import { getContext } from 'svelte';
  import CheckIcon from '~icons/lucide/check';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    active?: boolean;
    children: Snippet;
    onclick: () => void;
  };

  let { style, active = false, children, onclick }: Props = $props();

  // In focus-managed menus hover moves focus, so hover itself must not paint a second highlight.
  const focusManaged = getContext<boolean>('dropdownMenuFocusManaged') ?? false;
</script>

<button
  class={cx(
    'group',
    css(
      {
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        gap: '16px',
        paddingX: '16px',
        paddingY: '8px',
        textAlign: 'left',
        fontSize: '13px',
        color: active ? 'text.brand' : 'text.default',
        _hover: focusManaged ? undefined : { color: 'text.brand', backgroundColor: 'surface.subtle' },
        _focus: { color: 'text.brand', backgroundColor: 'surface.subtle' },
      },
      style,
    ),
  )}
  data-active={active}
  {onclick}
  type="button"
>
  {@render children()}

  {#if active}
    <Icon icon={CheckIcon} size={16} />
  {/if}
</button>
