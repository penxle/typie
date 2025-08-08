<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import CheckIcon from '~icons/lucide/check';
  import { Icon } from '$lib/components';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { Snippet } from 'svelte';

  type Props = {
    style?: SystemStyleObject;
    active?: boolean;
    children: Snippet;
    onclick: () => void;
  };

  let { style, active = false, children, onclick }: Props = $props();
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
        _hover: { color: 'text.brand' },
      },
      style,
    ),
  )}
  {onclick}
  type="button"
>
  {@render children()}

  {#if active}
    <Icon icon={CheckIcon} size={16} />
  {/if}
</button>
