<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { getEditor } from '$lib/editor/context';

  const editor = getEditor();

  let overlayEl = $state<HTMLDivElement>();

  const handleOverlayContextMenu = (e: MouseEvent) => {
    if (!overlayEl) return;
    editor.handleOverlayContextMenu(e, overlayEl);
  };
</script>

{#if editor.contextMenu.isOpen}
  <div
    bind:this={overlayEl}
    class={css({ position: 'fixed', inset: '0', zIndex: '50' })}
    onclick={() => editor.closeContextMenu()}
    oncontextmenu={handleOverlayContextMenu}
    role="presentation"
  >
    <div
      style="left: {editor.contextMenu.x}px; top: {editor.contextMenu.y}px;"
      class={css({
        position: 'absolute',
        borderRadius: '6px',
        borderWidth: '1px',
        borderColor: 'border.default',
        backgroundColor: 'surface.default',
        paddingY: '2px',
        boxShadow: 'small',
      })}
      oncontextmenu={(e) => e.stopPropagation()}
      role="menu"
      tabindex="-1"
    >
      <button
        class={css({
          width: 'full',
          paddingX: '12px',
          paddingY: '6px',
          textAlign: 'left',
          fontSize: '12px',
          color: 'text.subtle',
          transition: 'colors',
          cursor: 'pointer',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        onclick={() => editor.handleCopy()}
        type="button"
      >
        복사
      </button>
      <button
        class={css({
          width: 'full',
          paddingX: '12px',
          paddingY: '6px',
          textAlign: 'left',
          fontSize: '12px',
          color: 'text.subtle',
          transition: 'colors',
          cursor: 'pointer',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        onclick={() => editor.handleCut()}
        type="button"
      >
        잘라내기
      </button>
      <button
        class={css({
          width: 'full',
          paddingX: '12px',
          paddingY: '6px',
          textAlign: 'left',
          fontSize: '12px',
          color: 'text.subtle',
          transition: 'colors',
          cursor: 'pointer',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        onclick={() => editor.handlePaste()}
        type="button"
      >
        붙여넣기
      </button>
      <div class={css({ marginY: '2px', height: '1px', backgroundColor: 'border.default' })}></div>
      <button
        class={css({
          width: 'full',
          paddingX: '12px',
          paddingY: '6px',
          textAlign: 'left',
          fontSize: '12px',
          color: 'text.subtle',
          transition: 'colors',
          cursor: 'pointer',
          _hover: { backgroundColor: 'interactive.hover' },
        })}
        onclick={() => editor.handleSelectAll()}
        type="button"
      >
        전체 선택
      </button>
    </div>
  </div>
{/if}
