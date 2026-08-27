<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Icon, Menu, MenuItem } from '@typie/ui/components';
  import SendIcon from '~icons/lucide/arrow-up';
  import BookOpenIcon from '~icons/lucide/book-open';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ShieldCheckIcon from '~icons/lucide/shield-check';
  import StopIcon from '~icons/lucide/square';
  import ZapIcon from '~icons/lucide/zap';
  import { commandGate, commandsMatching } from './lib/commands.ts';
  import { swap } from './lib/motion.ts';
  import type { ToolPolicy } from '@typie/prism';
  import type { Component } from 'svelte';
  import type { PrismCommand } from './lib/commands.ts';

  type Props = {
    running: boolean;
    sendDisabled: boolean;
    blocked: boolean;
    commands: PrismCommand[] | null;
    status: { text: string; stop: string | null } | null;
    policy: { current: ToolPolicy; onChange: (policy: ToolPolicy) => void };
    onSend: (text: string) => Promise<void>;
    onStop: () => Promise<void>;
    text?: string;
  };

  let { running, sendDisabled, blocked, commands, status, policy, onSend, onStop, text = $bindable('') }: Props = $props();

  const policyOptions: { value: ToolPolicy; label: string; description: string; icon: Component }[] = [
    { value: 'READ_ONLY', label: '읽기 전용', description: '스페이스를 읽기만 하고 바꾸지 않아요.', icon: BookOpenIcon },
    { value: 'STANDARD', label: '중요한 일만 확인', description: '지우거나 공개 범위를 바꿀 때만 먼저 물어봐요.', icon: ShieldCheckIcon },
    { value: 'FULL', label: '자동 실행', description: '묻지 않고 바로 실행해요.', icon: ZapIcon },
  ];
  const currentPolicy = $derived(policyOptions.find((option) => option.value === policy.current) ?? policyOptions[1]);

  const policyAnchorStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
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
    _disabled: { color: 'text.disabled!', backgroundColor: 'transparent!' },
  });

  const popoverStyle = css.raw({
    position: 'absolute',
    bottom: '[100%]',
    left: '12px',
    right: '12px',
    zIndex: '2',
    marginBottom: '4px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    backgroundColor: 'surface.default',
    boxShadow: 'menu',
  });

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
  let commandError = $state(false);
  let textarea = $state<HTMLTextAreaElement>();
  let boxEl = $state<HTMLElement>();
  let heightFrom = $state<number>();
  let prevStatusMode: boolean | undefined;

  $effect.pre(() => {
    const mode = status !== null;
    if (prevStatusMode !== undefined && mode !== prevStatusMode) heightFrom = boxEl?.offsetHeight;
    prevStatusMode = mode;
  });

  let slashDismissed = $state(false);
  let slashHighlight = $state(0);
  let slashListEl = $state<HTMLElement>();

  export const focus = () => textarea?.focus();

  const slashOpen = $derived(!slashDismissed && text.startsWith('/') && !/\s/.test(text) && commands !== null && commands.length > 0);
  const slashHits = $derived(commands === null ? [] : commandsMatching(commands, text.slice(1)));
  const slashVisible = $derived(slashOpen && slashHits.length > 0);
  const slashIndex = $derived(slashHits.length === 0 ? -1 : Math.min(slashHighlight, slashHits.length - 1));
  const unknownCommand = $derived(commandGate(text.trim(), commands) === 'unknown');

  $effect(() => {
    void slashHits;
    slashHighlight = 0;
  });

  $effect(() => {
    void text;
    commandError = false;
  });

  $effect(() => {
    if (slashIndex < 0) return;
    slashListEl?.querySelector(`[data-index="${slashIndex}"]`)?.scrollIntoView({ block: 'nearest' });
  });

  const moveHighlight = (delta: number) => {
    if (slashHits.length === 0) return;
    slashHighlight = Math.min(slashHits.length - 1, Math.max(0, slashIndex + delta));
  };

  const submit = async () => {
    const value = text.trim();

    if (busy || running || sendDisabled || blocked || value.length === 0) {
      return;
    }

    if (unknownCommand) {
      commandError = true;
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

  const commandText = (command: PrismCommand) => `/${command.name}${command.argumentHint === null ? '' : ' '}`;

  const adoptCommand = (command: PrismCommand) => {
    text = commandText(command);
    slashDismissed = true;
    textarea?.focus();
  };

  const onKeydown = (e: KeyboardEvent) => {
    if (e.isComposing) {
      return;
    }

    if (slashVisible && e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      slashDismissed = true;
      return;
    }

    if (slashVisible && (e.key === 'ArrowDown' || e.key === 'ArrowUp')) {
      e.preventDefault();
      moveHighlight(e.key === 'ArrowDown' ? 1 : -1);
      return;
    }

    if (slashVisible && !e.shiftKey && e.key === 'Tab') {
      e.preventDefault();
      adoptCommand(slashHits[slashIndex]);
      return;
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      if (slashVisible) {
        const target = slashHits[slashIndex];
        if (commandText(target) !== text) {
          adoptCommand(target);
          return;
        }
      }
      slashDismissed = true;
      void submit();
    }
  };
</script>

<div class={css({ position: 'relative', paddingX: '12px', paddingBottom: '8px' })}>
  {#if status === null && slashVisible}
    <div bind:this={slashListEl} class={css(popoverStyle, { padding: '4px', maxHeight: '240px', overflowY: 'auto' })} role="listbox">
      {#each slashHits as command, index (command.name)}
        <button
          class={flex({
            width: 'full',
            gap: '8px',
            paddingX: '8px',
            paddingY: '6px',
            borderRadius: '6px',
            fontSize: '12px',
            '&[data-highlighted="true"]': { backgroundColor: 'surface.muted' },
          })}
          aria-selected={index === slashIndex}
          data-highlighted={index === slashIndex}
          data-index={index}
          onclick={() => adoptCommand(command)}
          onpointermove={() => (slashHighlight = index)}
          role="option"
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
    bind:this={boxEl}
    class={css(
      flex.raw({
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
      }),
    )}
  >
    {#if status !== null}
      <div class={flex({ alignItems: 'center', gap: '8px', minHeight: '44px' })} in:swap={{ box: boxEl, from: heightFrom }}>
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
      <div class={flex({ flexDirection: 'column', gap: '6px' })} in:swap={{ box: boxEl, from: heightFrom }}>
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
          disabled={blocked}
          oninput={() => (slashDismissed = false)}
          onkeydown={onKeydown}
          placeholder={blocked ? '확인을 마치면 이어서 대화할 수 있어요' : '메시지를 입력하세요'}
          rows={1}
          bind:value={text}
          use:autosize={{ value: text }}></textarea>

        <div class={flex({ alignItems: 'center', gap: '8px', minHeight: '28px' })}>
          <Menu style={policyAnchorStyle} offset={8} placement="top-start">
            {#snippet button()}
              <Icon style={css.raw({ color: 'text.subtle' })} icon={currentPolicy.icon} size={14} />
              <span>{currentPolicy.label}</span>
              <Icon icon={ChevronDownIcon} size={12} />
            {/snippet}

            {#each policyOptions as option (option.value)}
              <MenuItem icon={option.icon} onclick={() => policy.onChange(option.value)}>
                <div class={flex({ flexDirection: 'column', gap: '2px' })}>
                  <span>{option.label}</span>
                  <span class={css({ fontSize: '11px', fontWeight: 'medium', color: 'text.faint' })}>{option.description}</span>
                </div>

                {#if option.value === policy.current}
                  <Icon style={css.raw({ marginLeft: 'auto', color: 'text.brand' })} icon={CheckIcon} size={14} />
                {/if}
              </MenuItem>
            {/each}
          </Menu>

          {#if commandError}
            <span
              class={css({
                minWidth: '0',
                fontSize: '12px',
                color: 'text.faint',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
              })}
            >
              등록되지 않은 명령이에요
            </span>
          {/if}

          {#if running}
            <button class={css(stopButtonStyle, { marginLeft: 'auto' })} aria-label="중단" onclick={() => onStop()} type="button">
              <Icon icon={StopIcon} size={12} />
            </button>
          {:else}
            {@const empty = text.trim().length === 0 || blocked}
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
                _disabled: { backgroundColor: 'surface.muted', color: 'text.faint' },
              })}
              aria-label="보내기"
              disabled={sendDisabled || busy || empty}
              onclick={() => submit()}
              type="button"
            >
              <Icon icon={SendIcon} size={14} />
            </button>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>
