<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import { getAppContext } from '@typie/ui/context';
  import { onMount, tick, untrack } from 'svelte';
  import ArrowDownIcon from '~icons/lucide/arrow-down';
  import ArrowUpIcon from '~icons/lucide/arrow-up';
  import ReplaceIcon from '~icons/lucide/replace';
  import ReplaceAllIcon from '~icons/lucide/replace-all';
  import WholeWordIcon from '~icons/lucide/whole-word';
  import XIcon from '~icons/lucide/x';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';

  type Props = {
    editor: Ref<Editor>;
    close: () => void;
  };

  let { editor, close }: Props = $props();

  let findInputEl: HTMLInputElement;

  let findText = $state('');
  let replaceText = $state('');

  const app = getAppContext();

  $effect(() => {
    void findText;
    void app.preference.current.searchMatchWholeWord;

    untrack(() => {
      editor.current.commands.search(findText, { matchWholeWord: app.preference.current.searchMatchWholeWord });
    });
  });

  const getFindTextFromSelection = () => {
    const { selection } = editor.current.state;
    if (selection.from !== selection.to) {
      const selectionText = editor.current.state.doc.textBetween(selection.from, selection.to, ' ');
      findText = selectionText;
    }
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && e.code === 'KeyF') {
      e.preventDefault();

      getFindTextFromSelection();

      tick().then(() => {
        findInputEl.select();
      });
      return;
    }

    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
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
    getFindTextFromSelection();
    tick().then(() => {
      findInputEl.select();
    });

    return () => {
      editor.current.commands.clearSearch();
    };
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div
  class={flex({
    gap: '4px',
    padding: '8px',
    position: 'absolute',
    top: '0',
    right: '52px',
    zIndex: 'overEditor',
    backgroundColor: 'surface.default',
    borderRadius: '6px',
    boxShadow: 'small',
  })}
  onkeydown={handleKeydown}
  role="dialog"
  tabindex="-1"
>
  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
    <div
      class={css({
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
      })}
    >
      <input
        bind:this={findInputEl}
        class={css({
          paddingLeft: '8px',
          paddingRight: '32px',
          paddingY: '4px',
          width: '200px',
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
      <button
        class={css({
          position: 'absolute',
          right: '4px',
          size: '22px',
          padding: '3px',
          borderRadius: '4px',
          color: 'text.faint',
          backgroundColor: 'transparent',
          _hover: {
            backgroundColor: 'surface.muted',
          },
          _pressed: {
            color: 'accent.brand.default',
            backgroundColor: 'accent.brand.subtle',
          },
        })}
        aria-pressed={app.preference.current.searchMatchWholeWord}
        onclick={() => (app.preference.current.searchMatchWholeWord = !app.preference.current.searchMatchWholeWord)}
        type="button"
        use:tooltip={{ message: '어절 단위로 찾기' }}
      >
        <Icon icon={WholeWordIcon} size={16} />
      </button>
    </div>
    <input
      class={css({
        paddingX: '8px',
        paddingY: '4px',
        width: '200px',
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
    <div class={flex({ alignItems: 'center', height: '30px' })}>
      <div class={css({ flex: '1', paddingLeft: '4px', width: '60px', fontSize: '12px', fontWeight: 'medium', color: 'text.subtle' })}>
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

    <div class={flex({ alignItems: 'center', gap: '4px', height: '30px', justifyContent: 'space-between' })}>
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
      <button
        class={center({
          size: '24px',
          padding: '4px',
          borderRadius: '4px',
          color: 'text.faint',
          _hover: {
            backgroundColor: 'surface.muted',
          },
        })}
        onclick={close}
        type="button"
        use:tooltip={{ message: '닫기', keys: ['Esc'] }}
      >
        <Icon icon={XIcon} size={16} />
      </button>
    </div>
  </div>
</div>
