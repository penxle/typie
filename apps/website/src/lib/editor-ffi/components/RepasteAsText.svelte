<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import { getEditorContext } from '../editor.svelte';
  import { pageRectToClientRect } from '../geometry';
  import { getViewportOverlayContext } from './ViewportOverlay.svelte';

  const { editor } = getEditorContext();
  const viewportOverlay = getViewportOverlayContext();

  let show = $derived(editor !== undefined && !editor.readOnly && editor.lastHistoryTag?.type === 'paste_html');

  const point = $derived.by(() => {
    const cursor = editor?.cursor;
    if (!show || !editor || !cursor) {
      return null;
    }

    void viewportOverlay.change;
    const rect = pageRectToClientRect(editor, { page_idx: cursor.page_idx, rect: cursor.caret });
    if (!rect) return null;

    return { x: rect.left, y: rect.bottom + 4 };
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
    position: 'fixed',
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
      editor?.handleRepasteAsText();
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
