<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import ClipboardTypeIcon from '~icons/lucide/clipboard-type';
  import { getEditorContext } from '../editor.svelte';
  import { pageRectToClientRect, selectionHeadRect } from '../geometry';
  import { getViewportOverlayContext } from './ViewportOverlay.svelte';

  const { editor } = getEditorContext();
  const viewportOverlay = getViewportOverlayContext();

  let show = $derived(
    editor !== undefined &&
      !editor.readOnly &&
      editor.focused &&
      editor.selection !== undefined &&
      editor.lastHistoryTag?.type === 'paste_html',
  );

  const point = $derived.by(() => {
    if (!show || !editor) {
      return null;
    }

    void viewportOverlay.change;
    const snapshot = editor.published?.snapshot;
    const cursor = snapshot?.cursor;
    const anchor = cursor ? { page_idx: cursor.page_idx, rect: cursor.caret } : selectionHeadRect(snapshot);
    if (!anchor) return null;

    const rect = pageRectToClientRect(editor, anchor);
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
    borderColor: 'border.hairline',
    borderRadius: '6px',
    boxShadow: 'sm',
    fontSize: '13px',
    fontWeight: 'medium',
    color: 'text.muted',
    cursor: 'pointer',
    transition: 'colors',
    userSelect: 'none',
    whiteSpace: 'nowrap',
    zIndex: 'menu',
    _hover: {
      backgroundColor: 'surface.hover',
      color: 'text.default',
      borderColor: 'border.emphasis',
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
      e.preventDefault();
      e.stopPropagation();
    }}
    type="button"
  >
    <ClipboardTypeIcon />
    <span>서식 없이 다시 붙여넣기</span>
  </button>
{/if}
