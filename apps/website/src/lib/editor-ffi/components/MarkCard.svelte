<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import CopyIcon from '~icons/lucide/copy';
  import ExternalLinkIcon from '~icons/lucide/external-link';
  import LinkIcon from '~icons/lucide/link';
  import PencilIcon from '~icons/lucide/pencil';
  import UnlinkIcon from '~icons/lucide/unlink';
  import RubyIcon from '~icons/typie/ruby';
  import RubyOffIcon from '~icons/typie/ruby-off';
  import MarkBar from './MarkBar.svelte';
  import type { Component } from 'svelte';

  type Props = {
    kind: 'link' | 'ruby';
    value: string;
    editing: boolean;
    canEdit: boolean;
    canCopy: boolean;
    autofocus?: boolean;
    onopen?: () => void;
    oncopy?: () => Promise<void>;
    onedit: () => void;
    onremove: () => void;
    onsubmit: (value: string) => void;
    oncancel?: () => void;
  };

  let { kind, value, editing, canEdit, canCopy, autofocus = false, onopen, oncopy, onedit, onremove, onsubmit, oncancel }: Props = $props();

  let copied = $state(false);
  let root = $state<HTMLElement>();

  onMount(() => {
    if (!autofocus || editing) return;
    const timer = setTimeout(() => root?.querySelector<HTMLElement>('button')?.focus({ preventScroll: true }), 0);
    return () => clearTimeout(timer);
  });

  const iconButton = center({
    flexShrink: '0',
    size: '28px',
    borderRadius: '6px',
    color: 'text.muted',
    transition: 'common',
    _hover: { color: 'text.default', backgroundColor: 'surface.hover' },
  });

  const dangerButton = center({
    flexShrink: '0',
    size: '28px',
    borderRadius: '6px',
    color: 'text.muted',
    transition: 'common',
    _hover: { color: 'danger.default', backgroundColor: 'surface.hover' },
  });
</script>

{#snippet action(label: string, icon: Component, onclick: () => void, className: string, message?: string)}
  <button
    class={className}
    aria-label={label}
    {onclick}
    onpointerdown={(e) => e.preventDefault()}
    type="button"
    use:tooltip={{ message: message ?? label, arrow: false, keepOnClick: message !== undefined }}
  >
    <Icon {icon} size={14} />
  </button>
{/snippet}

<div bind:this={root}>
  {#if editing}
    <MarkBar initialValue={value} {kind} {oncancel} {onsubmit} />
  {:else}
    <div class={flex({ alignItems: 'center', gap: '8px', height: '36px', paddingLeft: '12px', paddingRight: '4px' })}>
      <Icon style={css.raw({ flexShrink: '0', color: 'text.hint' })} icon={kind === 'link' ? LinkIcon : RubyIcon} size={14} />
      <span
        class={css({ flexGrow: '1', minWidth: '0', maxWidth: '240px', fontSize: '13px', color: 'text.muted', truncate: true })}
        title={value}
      >
        {value}
      </span>

      <div class={flex({ alignItems: 'center', gap: '2px', flexShrink: '0' })}>
        {#if kind === 'link' && onopen}
          {@render action('링크 열기', ExternalLinkIcon, onopen, iconButton)}
        {/if}
        {#if kind === 'link' && canCopy && oncopy}
          {@render action(
            '링크 복사',
            CopyIcon,
            async () => {
              await oncopy();
              copied = true;
            },
            iconButton,
            copied ? '복사되었어요' : '링크 복사',
          )}
        {/if}
        {#if canEdit}
          {@render action('편집', PencilIcon, onedit, iconButton)}
          {@render action(kind === 'link' ? '링크 제거' : '루비 제거', kind === 'link' ? UnlinkIcon : RubyOffIcon, onremove, dangerButton)}
        {/if}
      </div>
    </div>
  {/if}
</div>
