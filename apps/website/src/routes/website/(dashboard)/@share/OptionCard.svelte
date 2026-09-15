<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    icon: Component;
    label: string;
    description?: string;
    selected: boolean;
    disabled?: boolean;
    trailing?: 'radio' | 'chevron';
    hint?: string | null;
    onclick: () => void;
    body?: Snippet;
  };

  let { icon, label, description, selected, disabled = false, trailing = 'radio', hint = null, onclick, body }: Props = $props();
</script>

<div
  class={css({
    borderWidth: '1px',
    borderColor: selected ? 'accent.default' : 'border.hairline',
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    transition: 'common',
    _hover: { borderColor: selected || disabled ? undefined : 'border.emphasis' },
  })}
>
  <button
    class={flex({
      alignItems: 'center',
      gap: '12px',
      width: 'full',
      paddingX: '12px',
      paddingY: '11px',
      textAlign: 'left',
      _disabled: { opacity: '45', cursor: 'not-allowed' },
      _hover: { '& .option-chevron': { color: 'text.muted', translateX: '2px' } },
    })}
    aria-checked={trailing === 'radio' ? selected : undefined}
    {disabled}
    {onclick}
    role={trailing === 'radio' ? 'radio' : undefined}
    type="button"
  >
    <span
      class={center({
        flexShrink: '0',
        size: '32px',
        borderRadius: '8px',
        color: selected ? 'text.default' : 'text.muted',
        backgroundColor: selected ? 'surface.active' : 'surface.inset',
        transition: 'common',
      })}
    >
      <Icon {icon} size={16} />
    </span>

    <span class={flex({ flexDirection: 'column', gap: '2px', flex: '1', minWidth: '0' })}>
      <span class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{label}</span>
      {#if description}
        <span class={css({ overflow: 'hidden', fontSize: '12px', textOverflow: 'ellipsis', color: 'text.hint' })}>{description}</span>
      {/if}
    </span>

    {#if hint}
      <span class={css({ flexShrink: '0', fontSize: '12px', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>{hint}</span>
    {:else if trailing === 'chevron'}
      <span class={cx('option-chevron', center({ flexShrink: '0', color: 'text.hint', translate: 'auto', transition: 'common' }))}>
        <Icon icon={ChevronRightIcon} size={16} />
      </span>
    {:else}
      <span
        class={center({
          flexShrink: '0',
          size: '16px',
          borderWidth: '[1.5px]',
          borderColor: selected ? 'accent.default' : 'border.emphasis',
          borderRadius: 'full',
          transition: 'common',
        })}
      >
        {#if selected}
          <span class={css({ size: '8px', borderRadius: 'full', backgroundColor: 'accent.default' })}></span>
        {/if}
      </span>
    {/if}
  </button>

  {#if body && selected}
    <div
      class={flex({
        flexDirection: 'column',
        gap: '14px',
        paddingX: '12px',
        paddingY: '14px',
        borderTopWidth: '1px',
        borderColor: 'border.hairline',
      })}
    >
      {@render body()}
    </div>
  {/if}
</div>
