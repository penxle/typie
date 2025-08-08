<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
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
      };
    };
  };

  let { open = $bindable(), title, children, footer, actions }: Props = $props();

  const handleBackdropClick = (e: MouseEvent) => {
    if (e.target === e.currentTarget) {
      open = false;
    }
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      open = false;
    }
  };
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
      backgroundColor: '[rgba(0, 0, 0, 0.8)]',
      zIndex: '[1000]',
    })}
    onclick={handleBackdropClick}
    onkeydown={handleKeydown}
    role="button"
    tabindex="-1"
  >
    <div
      class={css({
        position: 'relative',
        width: 'full',
        maxWidth: '[500px]',
        margin: '20px',
        backgroundColor: 'gray.900',
        borderWidth: '2px',
        borderColor: 'amber.500',
        fontFamily: 'mono',
        boxShadow: '[0 0 20px rgba(251, 191, 36, 0.3)]',
      })}
    >
      {#if title}
        <div
          class={flex({
            alignItems: 'center',
            justifyContent: 'space-between',
            borderBottomWidth: '2px',
            borderColor: 'amber.500',
            paddingX: '20px',
            paddingY: '16px',
            backgroundColor: 'gray.900',
          })}
        >
          <h2 class={css({ fontSize: '14px', fontWeight: 'bold', color: 'amber.500', letterSpacing: '0.05em' })}>
            {title.toUpperCase()}
          </h2>
          <button
            class={css({
              padding: '6px',
              color: 'gray.900',
              backgroundColor: 'amber.500',
              borderWidth: '1px',
              borderColor: 'amber.500',
              cursor: 'pointer',
              transition: 'common',
              _hover: {
                backgroundColor: 'amber.400',
                borderColor: 'amber.400',
              },
            })}
            onclick={() => (open = false)}
            type="button"
          >
            <AdminIcon style={{ color: 'gray.900' }} icon={XIcon} size={16} />
          </button>
        </div>
      {/if}

      <div
        class={css({
          padding: '20px',
          color: 'amber.500',
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
            borderColor: 'amber.500',
            paddingX: '20px',
            paddingY: '16px',
            backgroundColor: 'gray.900',
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
            borderColor: 'amber.500',
            paddingX: '20px',
            paddingY: '16px',
            backgroundColor: 'gray.900',
          })}
        >
          {#if actions.cancel}
            <button
              class={css({
                paddingX: '16px',
                paddingY: '6px',
                fontSize: '12px',
                fontWeight: 'medium',
                color: 'gray.900',
                backgroundColor: 'amber.500',
                borderWidth: '1px',
                borderColor: 'amber.500',
                cursor: 'pointer',
                transition: 'common',
                _hover: {
                  backgroundColor: 'amber.400',
                  borderColor: 'amber.400',
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
                color: actions.confirm.variant === 'danger' ? 'red.500' : 'amber.500',
                backgroundColor: 'gray.900',
                borderWidth: '1px',
                borderColor: actions.confirm.variant === 'danger' ? 'red.500' : 'amber.500',
                cursor: 'pointer',
                transition: 'common',
                _hover: {
                  backgroundColor: actions.confirm.variant === 'danger' ? 'red.500' : 'amber.500',
                  color: 'gray.900',
                  borderColor: actions.confirm.variant === 'danger' ? 'red.500' : 'amber.500',
                },
              })}
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
