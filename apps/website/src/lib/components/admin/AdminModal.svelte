<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import XIcon from '~icons/lucide/x';
  import { AdminIcon } from '$lib/components/admin';
  import type { Snippet } from 'svelte';

  type Props = {
    open: boolean;
    title?: string;
    children: Snippet;
    footer?: Snippet;
    // 기본 액션 버튼 props
    actions?: {
      cancel?: {
        label?: string;
        onclick?: () => void;
      };
      confirm?: {
        label?: string;
        onclick?: () => void;
        variant?: 'primary' | 'danger';
        disabled?: boolean;
      };
    };
  };

  let { open = $bindable(), title, children, footer, actions }: Props = $props();

  const handleBackdropClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      open = false;
    }
  };

  const handleBackdropKeydown = (e: KeyboardEvent) => {
    if (e.target === e.currentTarget && (e.key === 'Enter' || e.key === ' ')) {
      e.preventDefault();
      open = false;
    }
  };

  $effect(() => {
    if (!open) return;

    return pushEscapeHandler(() => {
      open = false;
      return true;
    });
  });
</script>

{#if open}
  <div
    class={flex({
      position: 'fixed',
      top: '0',
      left: '0',
      right: '0',
      bottom: '0',
      alignItems: 'center',
      justifyContent: 'center',
      backgroundColor: 'scrim',
      zIndex: '[1000]',
    })}
    onclick={handleBackdropClick}
    onkeydown={handleBackdropKeydown}
    role="button"
    tabindex="-1"
  >
    <div
      class={css({
        position: 'relative',
        width: 'full',
        maxWidth: '[500px]',
        margin: '20px',
        backgroundColor: 'surface.default',
        borderWidth: '2px',
        borderColor: 'border.default',
        fontFamily: 'mono',
        boxShadow: 'xl',
      })}
    >
      {#if title}
        <div
          class={flex({
            alignItems: 'center',
            justifyContent: 'space-between',
            borderBottomWidth: '2px',
            borderColor: 'border.default',
            paddingX: '20px',
            paddingY: '16px',
            backgroundColor: 'surface.default',
          })}
        >
          <h2 class={css({ fontSize: '14px', fontWeight: 'bold', color: 'text.default', letterSpacing: '0.05em' })}>
            {title.toUpperCase()}
          </h2>
          <button
            class={css({
              padding: '6px',
              color: 'text.muted',
              backgroundColor: 'transparent',
              borderWidth: '1px',
              borderColor: 'border.default',
              cursor: 'pointer',
              transition: 'common',
              _hover: {
                backgroundColor: 'surface.hover',
                borderColor: 'border.emphasis',
              },
            })}
            onclick={() => (open = false)}
            type="button"
          >
            <AdminIcon style={css.raw({ color: 'text.muted' })} icon={XIcon} size={16} />
          </button>
        </div>
      {/if}

      <div
        class={css({
          padding: '20px',
          color: 'text.default',
          fontSize: '12px',
          lineHeight: '[1.6]',
        })}
      >
        {@render children()}
      </div>

      {#if footer}
        <div
          class={flex({
            alignItems: 'center',
            justifyContent: 'flex-end',
            gap: '12px',
            borderTopWidth: '2px',
            borderColor: 'border.default',
            paddingX: '20px',
            paddingY: '16px',
            backgroundColor: 'surface.default',
          })}
        >
          {@render footer()}
        </div>
      {:else if actions}
        <div
          class={flex({
            alignItems: 'center',
            justifyContent: 'flex-end',
            gap: '12px',
            borderTopWidth: '2px',
            borderColor: 'border.default',
            paddingX: '20px',
            paddingY: '16px',
            backgroundColor: 'surface.default',
          })}
        >
          {#if actions.cancel}
            <button
              class={css({
                paddingX: '16px',
                paddingY: '6px',
                fontSize: '12px',
                fontWeight: 'medium',
                color: 'text.default',
                backgroundColor: 'surface.default',
                borderWidth: '1px',
                borderColor: 'border.default',
                cursor: 'pointer',
                transition: 'common',
                _hover: {
                  backgroundColor: 'surface.hover',
                  borderColor: 'border.emphasis',
                },
              })}
              onclick={actions.cancel.onclick || (() => (open = false))}
              type="button"
            >
              {actions.cancel.label || 'CANCEL'}
            </button>
          {/if}

          {#if actions.confirm}
            <button
              class={css({
                paddingX: '16px',
                paddingY: '6px',
                fontSize: '12px',
                fontWeight: 'medium',
                color: actions.confirm.variant === 'danger' ? 'text.on.danger' : 'surface.default',
                backgroundColor: actions.confirm.variant === 'danger' ? 'danger.default' : 'accent.default',
                borderWidth: '1px',
                borderColor: actions.confirm.variant === 'danger' ? 'danger.default' : 'accent.default',
                cursor: 'pointer',
                transition: 'common',
                _hover: {
                  backgroundColor:
                    actions.confirm.variant === 'danger'
                      ? '[color-mix(in oklch, token(colors.danger.default) 88%, black)]'
                      : '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
                  color: actions.confirm.variant === 'danger' ? 'text.on.danger' : 'surface.default',
                  borderColor:
                    actions.confirm.variant === 'danger'
                      ? '[color-mix(in oklch, token(colors.danger.default) 88%, black)]'
                      : '[color-mix(in oklch, token(colors.accent.default) 88%, black)]',
                },
                _disabled: {
                  opacity: '40',
                  cursor: 'not-allowed',
                  _hover: {
                    backgroundColor: actions.confirm.variant === 'danger' ? 'danger.default' : 'accent.default',
                    color: actions.confirm.variant === 'danger' ? 'text.on.danger' : 'surface.default',
                    borderColor: actions.confirm.variant === 'danger' ? 'danger.default' : 'accent.default',
                  },
                },
              })}
              disabled={actions.confirm.disabled ?? false}
              onclick={actions.confirm.onclick}
              type="button"
            >
              {actions.confirm.label || 'CONFIRM'}
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </div>
{/if}
