<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { prefersReducedMotion } from '@typie/ui/state';
  import { onDestroy, untrack } from 'svelte';
  import { flip } from 'svelte/animate';
  import { fly } from 'svelte/transition';
  import CloudOffIcon from '~icons/lucide/cloud-off';
  import LockIcon from '~icons/lucide/lock';
  import { paneChromeAttachment } from '../@pane/pane-chrome-attachment';
  import type { PaneChromeAttachmentHandle } from '../@pane/zen-mode-pane-chrome.svelte';

  type Props = {
    saveFailed: boolean;
    top: string;
    attachment: PaneChromeAttachmentHandle;
    onUnlock?: () => void;
    onShowSaveDetails: () => void;
  };
  type Notice = 'locked' | 'save-failed';

  let { saveFailed, top, attachment, onUnlock, onShowSaveDetails }: Props = $props();
  let notices = $state<Notice[]>([]);
  // eslint-disable-next-line svelte/prefer-svelte-reactivity -- only notices drive rendering
  const timers = new Map<Notice, ReturnType<typeof setTimeout>>();
  const content = $derived({
    locked: { icon: LockIcon, message: '편집이 잠겨있는 문서예요.', actionLabel: '해제하기', action: onUnlock },
    'save-failed': {
      icon: CloudOffIcon,
      message: '최근 변경사항을 저장하지 못했어요.',
      actionLabel: '저장 상태 확인',
      action: onShowSaveDetails,
    },
  });

  function hide(notice: Notice) {
    clearTimeout(timers.get(notice));
    timers.delete(notice);
    notices = notices.filter((entry) => entry !== notice);
  }

  function scheduleHide(notice: Notice) {
    clearTimeout(timers.get(notice));
    timers.set(
      notice,
      setTimeout(() => hide(notice), 5000),
    );
  }

  function show(notice: Notice) {
    if (notices.includes(notice)) return;
    notices = [...notices, notice];
    scheduleHide(notice);
  }

  export function showLocked() {
    show('locked');
  }

  $effect(() => {
    const failed = saveFailed;
    untrack(() => {
      if (failed) show('save-failed');
      else hide('save-failed');
    });
  });

  onDestroy(() => {
    for (const timer of timers.values()) clearTimeout(timer);
  });
</script>

<div
  style:top
  style:transition="var(--editor-pane-overlay-position-transition, none)"
  class={flex({
    position: 'absolute',
    right: '12px',
    zIndex: 'editorOverlay',
    direction: 'column',
    alignItems: 'flex-end',
    gap: '8px',
    maxWidth: '[calc(100% - 24px)]',
    pointerEvents: 'none',
  })}
  data-pane-chrome-reveal-exclusion
  use:paneChromeAttachment={attachment}
>
  {#each notices as notice (notice)}
    {@const item = content[notice]}
    <div
      class={flex({
        alignItems: 'center',
        gap: '10px',
        paddingX: '14px',
        paddingY: '10px',
        borderRadius: '6px',
        borderWidth: '1px',
        borderColor: 'border.default',
        backgroundColor: 'surface.default',
        boxShadow: 'sm',
        fontSize: '13px',
        color: 'text.muted',
        pointerEvents: 'auto',
      })}
      onpointerenter={() => clearTimeout(timers.get(notice))}
      onpointerleave={() => scheduleHide(notice)}
      role="alert"
      transition:fly={{ y: -8, duration: prefersReducedMotion.current ? 0 : 150 }}
      animate:flip={{ duration: prefersReducedMotion.current ? 0 : 180 }}
    >
      <Icon
        style={css.raw({ flexShrink: '0', color: notice === 'save-failed' ? 'danger.default' : undefined })}
        icon={item.icon}
        size={14}
      />
      <span>{item.message}</span>
      {#if item.action}
        <button
          class={css({
            marginLeft: '4px',
            paddingX: '8px',
            paddingY: '4px',
            borderRadius: '4px',
            fontSize: '12px',
            fontWeight: 'medium',
            color: 'text.default',
            backgroundColor: 'surface.canvas',
            cursor: 'pointer',
            transition: 'common',
            _hover: { backgroundColor: 'surface.hover' },
          })}
          onclick={() => {
            item.action?.();
            hide(notice);
          }}
          type="button"
        >
          {item.actionLabel}
        </button>
      {/if}
    </div>
  {/each}
</div>
