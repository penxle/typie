<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import ReplaceIcon from '~icons/lucide/replace';
  import ReplaceAllIcon from '~icons/lucide/replace-all';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { createFindReplaceManager } from '$lib/editor/find-replace';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { FindReplaceManager } from '$lib/editor/find-replace';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
    close: () => void;
  };

  let { editor, close }: Props = $props();

  let findInput: HTMLInputElement;
  let findText = $state('');
  let replaceText = $state('');
  let manager: FindReplaceManager | null = $state(null);
  let currentMatch = $state(0);
  let totalMatches = $state(0);

  const updateMatches = () => {
    if (!manager) return;

    const result = manager.search(findText);
    currentMatch = result.currentIndex;
    totalMatches = result.results.length;
  };

  const findNext = () => {
    if (!manager || totalMatches === 0) return;
    currentMatch = manager.next();
  };

  const findPrevious = () => {
    if (!manager || totalMatches === 0) return;
    currentMatch = manager.previous();
  };

  const replace = () => {
    if (!manager || totalMatches === 0) return;
    const result = manager.replace(replaceText);
    if (result.success) {
      currentMatch = result.currentIndex;
      totalMatches = manager.getResults().length;
    }
  };

  const replaceAll = () => {
    if (!manager || !findText) return;
    manager.replaceAll(replaceText);
    updateMatches();
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close();
      editor.current.commands.focus(undefined, { scrollIntoView: false });
    }
  };

  const handleKeydownInFindInput = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.isComposing) {
      if (e.shiftKey) {
        findPrevious();
      } else {
        findNext();
      }
    }
  };

  const handleKeydownInReplaceInput = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.isComposing) {
      if (e.metaKey) {
        replaceAll();
      } else {
        replace();
      }
    }
  };

  // Create manager when component mounts
  $effect(() => {
    if (!manager && editor.current) {
      manager = createFindReplaceManager(editor.current);
    }
  });

  onMount(() => {
    // 다음 틱에서 포커스 설정
    tick().then(() => {
      findInput?.focus();
    });
  });

  onDestroy(() => {
    manager?.clear();
  });

  $effect(() => {
    updateMatches();
  });
</script>

<div
  class={flex({
    gap: '4px',
    padding: '8px',
    width: '320px',
  })}
  onkeydown={handleKeydown}
  role="dialog"
  tabindex="-1"
>
  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <input
      bind:this={findInput}
      id="find-input"
      class={css({
        paddingX: '8px',
        paddingY: '4px',
        height: '30px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        fontSize: '14px',
      })}
      aria-label="찾을 텍스트"
      onkeydown={handleKeydownInFindInput}
      placeholder="찾기"
      type="text"
      bind:value={findText}
    />
    <input
      id="replace-input"
      class={css({
        paddingX: '8px',
        paddingY: '4px',
        height: '30px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '6px',
        fontSize: '14px',
      })}
      aria-label="바꿀 텍스트"
      onkeydown={handleKeydownInReplaceInput}
      placeholder="바꾸기"
      type="text"
      bind:value={replaceText}
    />
  </div>

  <div class={flex({ flex: '1', flexDirection: 'column', gap: '4px' })}>
    <div class={flex({ alignItems: 'center', gap: '4px', height: '30px' })}>
      <div class={css({ flex: '1', fontSize: '12px', color: 'text.subtle' })}>
        {#if totalMatches > 0}
          {currentMatch + 1} / {totalMatches}
        {:else}
          결과 없음
        {/if}
      </div>

      <div class={flex({ gap: '4px' })}>
        <button
          class={css({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={findPrevious}
          type="button"
          use:tooltip={{
            message: '이전 결과 찾기',
            keys: ['Shift', 'Enter'],
          }}
        >
          <Icon icon={ArrowUpIcon} size={16} />
        </button>
        <button
          class={center({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={findNext}
          type="button"
          use:tooltip={{
            message: '다음 결과 찾기',
            keys: ['Enter'],
          }}
        >
          <Icon icon={ArrowDownIcon} size={16} />
        </button>
      </div>
    </div>
    <div class={flex({ alignItems: 'center', gap: '4px', height: '30px' })}>
      <div class={flex({ alignItems: 'center', gap: '4px' })}>
        <button
          class={center({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={() => replace()}
          type="button"
          use:tooltip={{
            message: '바꾸기',
            keys: ['Enter'],
          }}
        >
          <Icon icon={ReplaceIcon} size={16} />
        </button>
        <button
          class={center({
            size: '24px',
            padding: '4px',
            borderRadius: '4px',
            color: 'text.faint',
            _hover: {
              backgroundColor: 'surface.muted',
            },
            _disabled: {
              opacity: '[0.5]',
              _hover: {
                backgroundColor: 'transparent',
              },
            },
          })}
          disabled={!findText}
          onclick={replaceAll}
          type="button"
          use:tooltip={{
            message: '모두 바꾸기',
            keys: ['Mod', 'Enter'],
          }}
        >
          <Icon icon={ReplaceAllIcon} size={16} />
        </button>
      </div>
    </div>
  </div>
</div>
