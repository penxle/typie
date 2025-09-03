<script lang="ts">
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { onMount } from 'svelte';
  import ShapesIcon from '~icons/lucide/shapes';
  import type { Editor } from '@tiptap/core';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  let isTemplateActive = $state(!window.__webview__ || editor.storage.webviewFeatures?.includes('template'));

  onMount(() => {
    const updateHandler = () => {
      isTemplateActive = !window.__webview__ || editor.storage.webviewFeatures?.includes('template');
    };

    editor.on('transaction', updateHandler);

    return () => {
      editor.off('transaction', updateHandler);
    };
  });

  const onTemplateClick = () => {
    if (window.__webview__) {
      window.__webview__?.emitEvent('useTemplate');
    } else {
      window.dispatchEvent(new CustomEvent('open-template-modal'));
    }
  };
</script>

<div
  class={flex({
    position: 'relative',
    flexDirection: 'column',
    gap: '4px',
    color: 'text.disabled',
    pointerEvents: 'none',
  })}
>
  <div>내용을 입력하거나 /를 입력해 블록 삽입하기...</div>

  {#if isTemplateActive}
    <div class={flex({ alignItems: 'center', gap: '4px' })}>
      <div>혹은</div>
      <button
        class={flex({
          alignItems: 'center',
          gap: '4px',
          transition: 'common',
          pointerEvents: 'auto',
          _hover: { color: 'text.faint' },
        })}
        onclick={onTemplateClick}
        type="button"
      >
        <Icon icon={ShapesIcon} size={16} />
        <div>템플릿 사용하기</div>
      </button>
    </div>
  {/if}
</div>
