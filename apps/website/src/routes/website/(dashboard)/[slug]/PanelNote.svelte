<script lang="ts">
  import getCaretCoordinates from 'textarea-caret';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { YState } from './state.svelte';
  import type * as Y from 'yjs';

  type Props = {
    doc: Y.Doc;
  };

  let { doc }: Props = $props();

  const note = new YState(doc, 'note', '');

  let element = $state<HTMLTextAreaElement>();
  const paddingBottom = $derived(element ? Number.parseFloat(getComputedStyle(element).lineHeight) * 2 : 0);

  const scroll = () => {
    if (!element) {
      return;
    }

    const { top, height } = getCaretCoordinates(element, element.selectionEnd);
    const caretBottom = top + height;

    const visibleTop = element.scrollTop;
    const visibleBottom = visibleTop + element.clientHeight;

    if (caretBottom + paddingBottom > visibleBottom) {
      element.scrollTop = Math.min(caretBottom - element.clientHeight + paddingBottom, element.scrollHeight - element.clientHeight);
      return;
    }

    if (top - paddingBottom < visibleTop) {
      element.scrollTop = Math.max(top - paddingBottom, 0);
    }
  };
</script>

<div
  class={flex({
    flexDirection: 'column',
    gap: '16px',
    flexGrow: '1',
    borderTopWidth: '1px',
    borderTopColor: 'gray.100',
    paddingTop: '16px',
  })}
>
  <div class={flex({ justifyContent: 'space-between', alignItems: 'center', paddingX: '20px' })}>
    <div class={css({ fontSize: '13px', fontWeight: 'semibold', color: 'gray.700' })}>노트</div>

    <!-- <button
      class={css({ fontSize: '13px', fontWeight: 'medium', color: 'gray.500', transition: 'common', _hover: { color: 'gray.700' } })}
      onclick={() => {}}
      type="button"
    >
      설정
    </button> -->
  </div>

  <textarea
    bind:this={element}
    class={css({
      flexGrow: '1',
      width: 'full',
      paddingX: '20px',
      paddingBottom: '20px',
      scrollBehavior: 'auto',
      overflowAnchor: 'auto',
      scrollPaddingY: '20px',
      fontSize: '13px',
      color: 'gray.700',
      wordBreak: 'break-all',
      resize: 'none',
    })}
    oninput={scroll}
    onkeydown={(e) => {
      if (e.key === 'ArrowUp' || e.key === 'ArrowDown') {
        requestAnimationFrame(scroll);
      }
    }}
    placeholder="포스트에 대해 기억할 내용이나 작성에 도움이 되는 내용을 적어둘 수 있어요. 모든 노트는 포스트 작성자만 볼 수 있어요."
    bind:value={note.current}
  ></textarea>
</div>
