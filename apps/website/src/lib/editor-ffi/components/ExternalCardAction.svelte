<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import type { Component } from 'svelte';

  type Props = {
    label: string;
    icon: Component;
    danger?: boolean;
    onclick: () => void;
  };

  let { label, icon, danger = false, onclick }: Props = $props();
</script>

<button
  class={css(
    center.raw({ flexShrink: '0', borderRadius: '4px', padding: '4px', transition: 'common' }),
    { color: 'text.muted' },
    danger
      ? { _hover: { backgroundColor: 'surface.hover', color: 'danger.default' } }
      : { _hover: { backgroundColor: 'surface.hover', color: 'text.default' } },
    { _focusVisible: { outlineWidth: '2px', outlineStyle: 'solid', outlineColor: 'accent.default', outlineOffset: '1px' } },
  )}
  aria-label={label}
  {onclick}
  onpointerdown={(event) => {
    event.preventDefault();
    event.stopPropagation();
  }}
  type="button"
  use:tooltip={{ message: label, arrow: false }}
>
  <Icon {icon} size={16} />
</button>
