<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon, RingSpinner } from '@typie/ui/components';
  import { EXTERNAL_CARD_HEIGHT } from '../external-element-height';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    icon: Component;
    title?: string;
    meta?: string;
    spinner?: boolean;
    loading?: boolean;
    progress?: number;
    label?: string;
    onclick?: () => void;
    children?: Snippet;
  };

  let { icon, title, meta, spinner = false, loading = false, progress, label, onclick, children }: Props = $props();

  // While metadata is still being resolved there is nothing truthful to print,
  // so show the shape the content will take instead of a placeholder sentence.
  const skeletonBar = css({
    backgroundColor: 'skeleton.base',
    borderRadius: '4px',
    animation: 'pulse 2s ease-in-out infinite',
  });

  const rowClass = flex({
    alignItems: 'center',
    gap: '12px',
    borderRadius: '8px',
    borderWidth: '1px',
    borderColor: 'border.default',
    paddingX: '12px',
    backgroundColor: 'surface.default',
    transition: 'common',
    _focusVisible: { outlineWidth: '2px', outlineStyle: 'solid', outlineColor: 'accent.default', outlineOffset: '2px' },
  });
</script>

<div class={css({ position: 'relative', borderRadius: '8px', overflow: 'hidden' })}>
  {#if onclick}
    <div
      style:min-height={`${EXTERNAL_CARD_HEIGHT}px`}
      class={cx(rowClass, css({ cursor: 'pointer', _hover: { borderColor: 'border.emphasis', backgroundColor: 'surface.hover' } }))}
      aria-label={label}
      {onclick}
      onkeydown={(event) => {
        if (event.key !== 'Enter' && event.key !== ' ') return;
        event.preventDefault();
        onclick();
      }}
      onpointerdown={(event) => event.stopPropagation()}
      role="button"
      tabindex="0"
    >
      {@render row()}
    </div>
  {:else}
    <div style:min-height={`${EXTERNAL_CARD_HEIGHT}px`} class={rowClass}>
      {@render row()}
    </div>
  {/if}

  {#if progress !== undefined}
    <div class={css({ position: 'absolute', bottom: '0', left: '0', right: '0', height: '2px', backgroundColor: 'surface.inset' })}>
      <div
        style:transform={`scaleX(${progress / 100})`}
        class={css({
          height: 'full',
          backgroundColor: 'accent.default',
          transformOrigin: 'left',
          transition: '[transform 140ms linear]',
        })}
      ></div>
    </div>
  {/if}
</div>

{#snippet row()}
  <div class={center({ flexShrink: '0', size: '36px', borderRadius: '6px', backgroundColor: 'surface.inset', color: 'text.muted' })}>
    {#if spinner}
      <RingSpinner style={css.raw({ size: '18px' })} />
    {:else}
      <Icon {icon} size={18} />
    {/if}
  </div>

  {#if loading}
    <div class={flex({ direction: 'column', flexGrow: '1', minWidth: '0', gap: '6px' })} aria-label="불러오는 중" role="status">
      <div style:width="42%" class={cx(skeletonBar, css({ height: '12px' }))}></div>
      <div style:width="24%" class={cx(skeletonBar, css({ height: '10px' }))}></div>
    </div>
  {:else}
    <div class={flex({ direction: 'column', flexGrow: '1', minWidth: '0', gap: '2px' })}>
      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default', truncate: true })} data-selection-label>
        {title}
      </span>
      {#if meta}
        <span class={css({ fontSize: '12px', color: 'text.muted', truncate: true })} data-selection-label>{meta}</span>
      {/if}
    </div>
  {/if}

  {@render children?.()}
{/snippet}
