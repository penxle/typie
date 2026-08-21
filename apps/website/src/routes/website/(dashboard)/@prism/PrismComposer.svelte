<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { fade } from 'svelte/transition';
  import SendIcon from '~icons/lucide/arrow-up';
  import StopIcon from '~icons/lucide/square';
  import { commandGate, commandsMatching } from './lib/commands.ts';
  import { fadeIn } from './lib/motion.ts';
  import type { PrismCommand } from './lib/commands.ts';

  type Props = {
    running: boolean;
    disabled: boolean;
    blocked: boolean;
    commands: PrismCommand[] | null;
    status: { text: string; stop: string | null } | null;
    onSend: (text: string) => Promise<void>;
    onStop: () => Promise<void>;
    text?: string;
  };

  let { running, disabled, blocked, commands, status, onSend, onStop, text = $bindable('') }: Props = $props();

  const stopButtonStyle = css.raw({
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
  });

  let busy = $state(false);
  let textarea = $state<HTMLTextAreaElement>();

  export const focus = () => textarea?.focus();

  const slashOpen = $derived(text.startsWith('/') && !/\s/.test(text) && commands !== null && commands.length > 0);
  const slashHits = $derived(commands === null ? [] : commandsMatching(commands, text.slice(1)));
  const unknownCommand = $derived(commandGate(text.trim(), commands) === 'unknown');

  const submit = async () => {
    const value = text.trim();

    if (busy || running || disabled || blocked || unknownCommand || value.length === 0) {
      return;
    }

    busy = true;
    text = '';

    try {
      await onSend(value);
    } catch {
      if (text.length === 0) {
        text = value;
      }
    } finally {
      busy = false;
      textarea?.focus();
    }
  };

  const adoptCommand = (command: PrismCommand) => {
    text = `/${command.name}${command.argumentHint === null ? '' : ' '}`;
  };

  const onKeydown = (e: KeyboardEvent) => {
    if (e.isComposing) {
      return;
    }

    if (slashOpen && !e.shiftKey && e.key === 'Tab' && slashHits.length > 0) {
      e.preventDefault();
      adoptCommand(slashHits[0]);
      return;
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (slashOpen && unknownCommand && slashHits.length > 0) {
        adoptCommand(slashHits[0]);
        return;
      }
      void submit();
    }
  };
</script>

<div class={css({ position: 'relative', paddingX: '12px', paddingBottom: '8px' })}>
  {#if status === null && slashOpen && slashHits.length > 0}
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
          onclick={() => adoptCommand(command)}
          type="button"
        >
          <span class={css({ fontWeight: 'semibold' })}>/{command.name}</span>
          <span class={css({ color: 'text.faint' })}>{command.description}</span>
          {#if command.argumentHint !== null}
            <span class={css({ marginLeft: 'auto', color: 'text.disabled' })}>{command.argumentHint}</span>
          {/if}
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
    {#if status !== null}
      <div class={flex({ alignItems: 'center', gap: '8px', minHeight: '44px' })} in:fade={fadeIn}>
        <p class={css({ flexGrow: '1', fontSize: '13px', color: 'text.subtle' })} role="status">{status.text}</p>
        {#if status.stop === null}
          <button class={css(stopButtonStyle)} aria-label="중단" onclick={() => onStop()} type="button">
            <Icon icon={StopIcon} size={12} />
          </button>
        {:else}
          <button
            class={css({
              flexShrink: '0',
              paddingX: '10px',
              paddingY: '5px',
              borderWidth: '1px',
              borderColor: 'border.default',
              borderRadius: '8px',
              fontSize: '12px',
              fontWeight: 'semibold',
              color: 'text.subtle',
              _hover: { backgroundColor: 'surface.muted', color: 'text.default' },
            })}
            onclick={() => onStop()}
            type="button"
          >
            {status.stop}
          </button>
        {/if}
      </div>
    {:else}
      {#if blocked}
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>확인을 마치면 이어서 대화할 수 있어요</p>
      {/if}

      {#if unknownCommand && slashHits.length === 0}
        <p class={css({ fontSize: '12px', color: 'text.faint' })}>등록되지 않은 명령이에요</p>
      {/if}

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
        disabled={disabled || blocked}
        onkeydown={onKeydown}
        placeholder="메시지를 입력하세요"
        rows={1}
        bind:value={text}
        use:autosize={{ value: text }}
        in:fade={fadeIn}></textarea>

      <div class={flex({ alignItems: 'center', gap: '8px' })} in:fade={fadeIn}>
        {#if running}
          <button class={css(stopButtonStyle, { marginLeft: 'auto' })} aria-label="중단" onclick={() => onStop()} type="button">
            <Icon icon={StopIcon} size={12} />
          </button>
        {:else}
          {@const empty = text.trim().length === 0 || blocked || unknownCommand}
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
    {/if}
  </div>
</div>
