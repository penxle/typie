<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import type { Snippet } from 'svelte';

  type Props = {
    actions?: Snippet;
    dividerVisible: boolean;
    height?: number;
    label: string;
    onToggle: () => void;
    open: boolean;
  };

  let { actions, dividerVisible, height = $bindable(0), label, onToggle, open }: Props = $props();
</script>

<div
  class={flex({
    position: 'sticky',
    top: '0',
    zIndex: '1',
    flexShrink: '0',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingX: '12px',
    paddingTop: '8px',
    paddingBottom: '4px',
    backgroundColor: 'surface.subtle',
    _after: {
      content: '""',
      position: 'absolute',
      right: '12px',
      bottom: '0',
      left: '12px',
      height: '1px',
      backgroundColor: 'border.subtle',
      opacity: dividerVisible ? '100' : '0',
      transition: '[opacity 150ms ease]',
    },
  })}
  bind:offsetHeight={height}
>
  <button
    class={flex({
      alignItems: 'center',
      gap: '4px',
      flexGrow: '1',
      minWidth: '0',
      height: '24px',
      paddingX: '8px',
      borderRadius: '4px',
      color: 'text.faint',
      opacity: '80',
      transition: 'common',
      _supportHover: { color: 'text.subtle', opacity: '100', '& > svg': { opacity: '100' } },
      _focusVisible: { opacity: '100', '& > svg': { opacity: '100' } },
    })}
    aria-expanded={open}
    onclick={onToggle}
    type="button"
  >
    <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{label}</span>
    <Icon
      style={css.raw({
        opacity: '0',
        transform: open ? 'rotate(90deg)' : 'rotate(0deg)',
        transition: '[opacity 120ms ease, transform 150ms cubic-bezier(0.23, 1, 0.32, 1)]',
        _motionReduce: { transition: '[none]' },
      })}
      icon={ChevronRightIcon}
      size={12}
    />
  </button>

  {@render actions?.()}
</div>
