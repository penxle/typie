<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { portal } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();

  const isTouchContextMenuOpen = $derived(editor.contextMenu.isOpen && editor.contextMenu.source === 'touch');
  const contextMenuPosition = $derived(isTouchContextMenuOpen ? { x: editor.contextMenu.x, y: editor.contextMenu.y } : null);
  const contextMenuPlacement = $derived(editor.contextMenu.placement);
  const touchMenuTransform = $derived(
    contextMenuPlacement.startsWith('top') ? 'translate(-50%, calc(-100% - 10px))' : 'translate(-50%, 10px)',
  );

  const closeTouchContextMenu = () => {
    editor.closeContextMenu();
    editor.focus();
  };

  const handleOutsidePointerDown = (event: PointerEvent) => {
    if (!isTouchContextMenuOpen) {
      return;
    }

    const target = event.target;
    if (!(target instanceof Element)) {
      return;
    }

    if (target.closest('[data-editor-touch-context-menu]')) {
      return;
    }

    const extensionAreaEl = editor.extensionArea.containerEl;
    if (extensionAreaEl?.contains(target)) {
      return;
    }

    editor.closeContextMenu();
  };
</script>

<svelte:window onpointerdowncapture={handleOutsidePointerDown} />

{#if isTouchContextMenuOpen && contextMenuPosition}
  <div
    style:left={`${contextMenuPosition.x}px`}
    style:top={`${contextMenuPosition.y}px`}
    style:transform={touchMenuTransform}
    class={css({
      position: 'fixed',
      zIndex: 'menu',
      pointerEvents: 'none',
    })}
    data-editor-touch-context-menu
    use:portal
  >
    <div
      class={css({
        display: 'flex',
        alignItems: 'center',
        gap: '2px',
        padding: '4px',
        borderWidth: '1px',
        borderRadius: 'full',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        pointerEvents: 'auto',
        userSelect: 'none',
        WebkitUserSelect: 'none',
        WebkitTouchCallout: 'none',
      })}
      transition:scale={{ start: 0.94, duration: 120 }}
    >
      <button
        class={css({
          appearance: 'none',
          border: 'none',
          background: 'transparent',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '36px',
          paddingX: '12px',
          borderRadius: 'full',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          whiteSpace: 'nowrap',
          WebkitTapHighlightColor: 'transparent',
          _active: {
            backgroundColor: 'surface.subtle',
          },
          _disabled: {
            opacity: '40',
            cursor: 'default',
          },
        })}
        disabled={editor.selection?.collapsed !== false}
        onclick={() => {
          void editor.handleCopy();
          closeTouchContextMenu();
        }}
        type="button"
      >
        복사
      </button>
      <div
        class={css({
          width: '1px',
          alignSelf: 'stretch',
          marginY: '6px',
          backgroundColor: 'border.default',
        })}
      ></div>
      <button
        class={css({
          appearance: 'none',
          border: 'none',
          background: 'transparent',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '36px',
          paddingX: '12px',
          borderRadius: 'full',
          fontSize: '13px',
          fontWeight: 'medium',
          color: 'text.default',
          whiteSpace: 'nowrap',
          WebkitTapHighlightColor: 'transparent',
          _active: {
            backgroundColor: 'surface.subtle',
          },
          _disabled: {
            opacity: '40',
            cursor: 'default',
          },
        })}
        onclick={() => {
          editor.handleSelectAll();
          closeTouchContextMenu();
        }}
        type="button"
      >
        전체 선택
      </button>
    </div>
  </div>
{/if}
