<script lang="ts">
  import CheckIcon from '~icons/lucide/check';
  import { Icon } from '$lib/components';
  import { css, cx } from '$styled-system/css';
  import type { Snippet } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

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
