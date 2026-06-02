<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import { getEditorContext } from '../editor.svelte';

  const ctx = getEditorContext();

  let show = $derived(ctx.editor !== undefined && !ctx.editor.readOnly && ctx.editor.lastHistoryTag?.type === 'paste_html');
  let point = $state<{ x: number; y: number } | null>(null);

  $effect(() => {
    const editor = ctx.editor;
    if (!show || !editor?.cursor) {
      point = null;
      return;
    }

    const { page_idx, caret } = editor.cursor;
    point = editor.localToOffset(page_idx, caret.x, caret.y + caret.height + 4);
  });

  $effect(() => {
    if (show) {
      return pushEscapeHandler(() => {
        show = false;
        return true;
      });
    }
  });

  const buttonStyle = css({
    position: 'absolute',
    display: 'flex',
    alignItems: 'center',
    gap: '4px',
    height: '28px',
    paddingX: '8px',
    backgroundColor: 'surface.default',
    border: '1px solid',
    borderColor: 'border.subtle',
    borderRadius: '6px',
    boxShadow: 'small',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.subtle',
    cursor: 'pointer',
    transition: 'colors',
    userSelect: 'none',
    whiteSpace: 'nowrap',
    zIndex: 'menu',
    _hover: {
      backgroundColor: 'surface.subtle',
      color: 'text.default',
      borderColor: 'border.default',
    },
  });
</script>

{#if point}
  <button
    style:left={`${point.x}px`}
    style:top={`${point.y}px`}
    class={buttonStyle}
    onclick={(e) => {
      e.stopPropagation();
      ctx.editor?.handleRepasteAsText();
    }}
    onpointerdown={(e) => {
      e.stopPropagation();
    }}
    type="button"
  >
    <ClipboardTypeIcon />
    <span>서식 없이 다시 붙여넣기</span>
  </button>
{/if}
