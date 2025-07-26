<script lang="ts">
  import { onMount, tick, untrack } from 'svelte';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import ReplaceIcon from '~icons/lucide/replace';
  import ReplaceAllIcon from '~icons/lucide/replace-all';
  import { tooltip } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor: Ref<Editor>;
    close: () => void;
  };

  let { editor, close }: Props = $props();

  let findInputEl: HTMLInputElement;

  let findText = $state('');
  let replaceText = $state('');

  $effect(() => {
    void findText;

    untrack(() => {
      editor.current.commands.search(findText);
    });
  });

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      close();
      editor.current.commands.focus(undefined, { scrollIntoView: false });
    }
  };

  const handleFindKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.isComposing) {
      if (e.shiftKey) {
        editor.current.commands.findPrevious();
      } else {
        editor.current.commands.findNext();
      }
    }
  };

  const handleReplaceKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && !e.isComposing) {
      if (e.metaKey) {
        editor.current.commands.replaceAll(replaceText);
      } else {
        editor.current.commands.replace(replaceText);
      }
    }
  };

  onMount(() => {
    tick().then(() => {
      findInputEl?.focus();
    });

    return () => {
      editor.current.commands.clearSearch();
    };
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
      bind:this={findInputEl}
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
      autocomplete="off"
      onkeydown={handleFindKeydown}
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
      autocomplete="off"
      onkeydown={handleReplaceKeydown}
      placeholder="바꾸기"
      type="text"
      bind:value={replaceText}
    />
  </div>

  <div class={flex({ flex: '1', flexDirection: 'column', gap: '4px' })}>
    <div class={flex({ alignItems: 'center', gap: '4px', height: '30px' })}>
      <div class={css({ flex: '1', fontSize: '12px', color: 'text.subtle' })}>
        {#if editor.current.extensionStorage.search.matches.length > 0}
          {editor.current.extensionStorage.search.currentIndex + 1} / {editor.current.extensionStorage.search.matches.length}
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
          onclick={() => editor.current.commands.findPrevious()}
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
          onclick={() => editor.current.commands.findNext()}
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
          onclick={() => editor.current.commands.replace(replaceText)}
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
          onclick={() => editor.current.commands.replaceAll(replaceText)}
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
