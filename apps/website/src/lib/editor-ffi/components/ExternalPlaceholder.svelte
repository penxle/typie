<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { EXTERNAL_CARD_HEIGHT } from '../external-element-height';
  import type { Component, Snippet } from 'svelte';

  type Props = {
    icon: Component;
    title: string;
    hint?: string;
    dropActive?: boolean;
    canEdit?: boolean;
    onclick?: () => void;
    children?: Snippet;
  };

  let { icon, title, hint, dropActive = false, canEdit = true, onclick, children }: Props = $props();
</script>

<div class={css({ position: 'relative' })}>
  <svelte:element
    this={canEdit ? 'button' : 'div'}
    style:min-height={`${EXTERNAL_CARD_HEIGHT}px`}
    class={css(
      flex.raw({
        alignItems: 'center',
        gap: '12px',
        width: 'full',
        borderRadius: '8px',
        borderWidth: '1px',
        borderStyle: 'dashed',
        borderColor: 'border.default',
        paddingX: '12px',
        textAlign: 'left',
        backgroundColor: 'surface.default',
        transition: 'common',
        _focusVisible: { outlineWidth: '2px', outlineStyle: 'solid', outlineColor: 'accent.default', outlineOffset: '2px' },
      }),
      canEdit && { _hover: { borderColor: 'border.emphasis', backgroundColor: 'surface.hover' } },
      dropActive && { borderColor: 'border.emphasis', backgroundColor: 'surface.hover' },
    )}
    aria-label={canEdit ? [title, hint].filter(Boolean).join(' — ') : undefined}
    onclick={canEdit ? onclick : undefined}
    onpointerdown={canEdit
      ? (event: PointerEvent) => {
          // The canvas turns a press on an external element into an engine interaction,
          // which republishes and swaps this node before the click lands.
          event.preventDefault();
          event.stopPropagation();
        }
      : undefined}
    role={canEdit ? undefined : 'group'}
    type={canEdit ? 'button' : undefined}
  >
    <div class={flex({ alignItems: 'center', gap: '12px', flexGrow: '1', minWidth: '0' })} aria-hidden={canEdit ? 'true' : undefined}>
      <div class={center({ flexShrink: '0', size: '36px', borderRadius: '6px', backgroundColor: 'surface.inset', color: 'text.hint' })}>
        <Icon {icon} size={18} />
      </div>
      <div class={flex({ direction: 'column', flexGrow: '1', minWidth: '0', gap: '2px' })}>
        <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.hint', truncate: true })}>{title}</span>
        {#if hint}
          <span class={css({ fontSize: '12px', color: 'text.muted', truncate: true })}>{hint}</span>
        {/if}
      </div>
    </div>
  </svelte:element>

  {#if children}
    <div
      class={css({
        position: 'absolute',
        top: '[50%]',
        right: '12px',
        transform: 'translateY(-50%)',
        opacity: '0',
        transition: 'common',
        pointerEvents: 'none',
        _groupActive: { opacity: '100', pointerEvents: 'auto' },
        _groupHover: { opacity: '100', pointerEvents: 'auto' },
        '&:focus-within': { opacity: '100', pointerEvents: 'auto' },
      })}
    >
      {@render children()}
    </div>
  {/if}
</div>
