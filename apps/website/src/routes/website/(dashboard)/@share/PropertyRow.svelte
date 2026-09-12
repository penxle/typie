<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import InfoIcon from '~icons/lucide/info';
  import { modifiedBadgeStyle } from './publish-styles';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    icon: Component;
    label: string;
    hint?: string;
    top?: boolean;
    modified?: boolean;
    children: Snippet;
  };

  let { icon, label, hint, top = false, modified = false, children }: Props = $props();

  let valueEl = $state<HTMLElement>();

  const forwardToControl = () => {
    const target =
      valueEl?.querySelector<HTMLElement>('[data-primary]:not([disabled])') ??
      valueEl?.querySelector<HTMLElement>('button:not([disabled]), input:not([disabled])');
    if (!target) return;

    if (target instanceof HTMLInputElement && target.type !== 'checkbox') {
      target.focus();
    } else {
      target.click();
    }
  };
</script>

<div
  class={cx(
    'group',
    css({
      display: 'grid',
      gridTemplateColumns: '[120px minmax(0, 1fr)]',
      gap: '4px',
      minHeight: '36px',
      paddingX: '8px',
      alignItems: top ? 'flex-start' : 'center',
    }),
  )}
>
  <button
    class={flex({
      alignItems: 'center',
      gap: '8px',
      height: '36px',
      fontSize: '13px',
      color: 'text.muted',
      textAlign: 'left',
      whiteSpace: 'nowrap',
      cursor: 'default',
    })}
    onclick={forwardToControl}
    tabindex="-1"
    type="button"
  >
    <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} {icon} size={14} />
    <span>{label}</span>

    {#if hint}
      <span class={center({ flexShrink: '0', color: 'text.hint' })} use:tooltip={{ message: hint, placement: 'top' }}>
        <Icon icon={InfoIcon} size={12} />
      </span>
    {/if}
  </button>

  <div bind:this={valueEl} class={flex({ alignItems: 'center', flexWrap: 'wrap', gap: '[4px 8px]', minWidth: '0', paddingY: '3px' })}>
    {@render children()}

    {#if modified}
      <span class={css(modifiedBadgeStyle)} use:tooltip={{ message: '아직 발행에 반영되지 않았어요', placement: 'top' }}>수정됨</span>
    {/if}
  </div>
</div>
