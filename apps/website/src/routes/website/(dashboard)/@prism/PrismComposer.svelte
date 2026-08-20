<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import SendIcon from '~icons/lucide/arrow-up';
  import StopIcon from '~icons/lucide/square';
  import { slashCommands } from './slash-commands';

  type Props = {
    running: boolean;
    disabled: boolean;
    onSend: (text: string) => Promise<void>;
    onStop: () => Promise<void>;
  };

  let { running, disabled, onSend, onStop }: Props = $props();

  let text = $state('');
  let busy = $state(false);
  let textarea = $state<HTMLTextAreaElement>();

  export const focus = () => textarea?.focus();

  const slashOpen = $derived(text.startsWith('/') && !text.includes(' ') && slashCommands.length > 0);
  const slashHits = $derived(slashCommands.filter((c) => c.name.startsWith(text.slice(1))));

  const submit = async () => {
    const value = text.trim();
    if (busy || running || disabled || value.length === 0) {
      return;
    }

    busy = true;
    text = '';

    try {
      await onSend(value);
    } catch {
      // 실패 문면은 onSend가 이미 띄웠다 — 새로 입력한 게 없을 때만 원문을 복원한다
      if (text.length === 0) {
        text = value;
      }
    } finally {
      busy = false;
      textarea?.focus();
    }
  };

  const onKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.shiftKey && !e.isComposing) {
      e.preventDefault();
      void submit();
    }
  };
</script>

<div class={css({ position: 'relative', paddingX: '12px', paddingBottom: '8px' })}>
  {#if slashOpen && slashHits.length > 0}
    <div
      class={css({
        position: 'absolute',
        bottom: '[100%]',
        left: '12px',
        right: '12px',
        marginBottom: '4px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '8px',
        backgroundColor: 'surface.default',
        boxShadow: 'menu',
        padding: '4px',
      })}
    >
      {#each slashHits as command (command.name)}
        <button
          class={flex({
            width: 'full',
            gap: '8px',
            paddingX: '8px',
            paddingY: '6px',
            borderRadius: '6px',
            fontSize: '12px',
            _hover: { backgroundColor: 'surface.muted' },
          })}
          onclick={() => (text = command.insert)}
          type="button"
        >
          <span class={css({ fontWeight: 'semibold' })}>/{command.name}</span>
          <span class={css({ color: 'text.faint' })}>{command.hint}</span>
        </button>
      {/each}
    </div>
  {/if}

  <div
    class={flex({
      flexDirection: 'column',
      gap: '6px',
      borderWidth: '1px',
      borderColor: 'border.default',
      borderRadius: '10px',
      backgroundColor: 'surface.default',
      _dark: { backgroundColor: 'surface.subtle' },
      boxShadow: 'small',
      transition: '[border-color 150ms ease]',
      _focusWithin: { borderColor: 'border.strong' },
      paddingX: '14px',
      paddingTop: '12px',
      paddingBottom: '10px',
    })}
  >
    <textarea
      bind:this={textarea}
      class={css({
        width: 'full',
        minHeight: '44px',
        maxHeight: '160px',
        fontSize: '13px',
        lineHeight: '[1.5]',
        resize: 'none',
        backgroundColor: 'transparent',
        outline: 'none',
        _disabled: { opacity: '50' },
      })}
      {disabled}
      onkeydown={onKeydown}
      placeholder="메시지를 입력하세요"
      rows={1}
      bind:value={text}
      use:autosize={{ value: text }}></textarea>

    <div class={flex({ alignItems: 'center', gap: '8px' })}>
      {#if running}
        <button
          class={css({
            marginLeft: 'auto',
            size: '28px',
            borderRadius: 'full',
            backgroundColor: 'surface.dark',
            color: 'text.bright',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: '0',
            transition: '[transform 160ms cubic-bezier(0.23, 1, 0.32, 1)]',
            _active: { transform: 'scale(0.97)' },
          })}
          aria-label="중단"
          onclick={() => onStop()}
          type="button"
        >
          <Icon icon={StopIcon} size={12} />
        </button>
      {:else}
        {@const empty = text.trim().length === 0}
        <button
          class={css({
            marginLeft: 'auto',
            size: '28px',
            borderRadius: 'full',
            backgroundColor: empty ? 'surface.muted' : 'accent.brand.default',
            color: empty ? 'text.faint' : 'text.bright',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            flexShrink: '0',
            transition: '[transform 160ms cubic-bezier(0.23, 1, 0.32, 1), background-color 150ms ease, color 150ms ease]',
            _active: { transform: 'scale(0.97)' },
          })}
          aria-label="보내기"
          disabled={disabled || busy || empty}
          onclick={() => submit()}
          type="button"
        >
          <Icon icon={SendIcon} size={14} />
        </button>
      {/if}
    </div>
  </div>
</div>
