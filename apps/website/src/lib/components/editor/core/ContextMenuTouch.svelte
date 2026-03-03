<script lang="ts">
  import { flip, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import { getEditorContext } from '$lib/editor/context.svelte';

  const { editor } = getEditorContext();
  const TOUCH_MENU_GAP = 10;
  const TOUCH_MENU_VIEWPORT_PADDING = 8;
  const TAP_FEEDBACK_MIN_MS = 70;

  const isTouchContextMenuOpen = $derived(editor.contextMenu.isOpen && editor.contextMenu.source === 'touch');
  const contextMenuPosition = $derived(isTouchContextMenuOpen ? { x: editor.contextMenu.x, y: editor.contextMenu.y } : null);
  const contextMenuPlacement = $derived(editor.contextMenu.placement);
  let pressedAction = $state<'copy' | 'selectAll' | null>(null);

  const { anchor: topAnchorAction, floating: topFloatingAction } = createFloatingActions({
    placement: 'top',
    offset: TOUCH_MENU_GAP,
    middleware: [shift({ padding: TOUCH_MENU_VIEWPORT_PADDING }), flip({ padding: TOUCH_MENU_VIEWPORT_PADDING })],
  });

  const { anchor: bottomAnchorAction, floating: bottomFloatingAction } = createFloatingActions({
    placement: 'bottom',
    offset: TOUCH_MENU_GAP,
    middleware: [shift({ padding: TOUCH_MENU_VIEWPORT_PADDING }), flip({ padding: TOUCH_MENU_VIEWPORT_PADDING })],
  });

  const closeTouchContextMenu = () => {
    pressedAction = null;
    editor.closeContextMenu();
    editor.focus();
  };

  const clearPressedAction = () => {
    pressedAction = null;
  };

  const waitForTapFeedback = async () => {
    await new Promise((resolve) => {
      setTimeout(resolve, TAP_FEEDBACK_MIN_MS);
    });
  };

  const runTouchMenuAction = async (action: 'copy' | 'selectAll', fn: () => void | Promise<void>) => {
    pressedAction = action;
    try {
      await fn();
    } finally {
      await waitForTapFeedback();
      closeTouchContextMenu();
    }
  };

  $effect(() => {
    if (!isTouchContextMenuOpen && pressedAction !== null) {
      pressedAction = null;
    }
  });

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

    clearPressedAction();
    editor.closeContextMenu();
  };
</script>

<svelte:window onpointerdowncapture={handleOutsidePointerDown} />

{#snippet touchContextMenuContent()}
  <div
    class={css({
      display: 'flex',
      alignItems: 'center',
      gap: '2px',
      padding: '2px',
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
      class={css(
        {
          appearance: 'none',
          border: 'none',
          background: 'transparent',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '36px',
          paddingX: '14px',
          borderRadius: 'full',
          fontSize: '16px',
          fontWeight: 'medium',
          color: 'text.default',
          whiteSpace: 'nowrap',
          WebkitTapHighlightColor: 'transparent',
          _active: {
            backgroundColor: 'surface.muted',
          },
          _disabled: {
            opacity: '40',
            cursor: 'default',
          },
        },
        pressedAction === 'copy' && { backgroundColor: 'surface.muted' },
      )}
      disabled={editor.selection?.collapsed !== false}
      onblur={clearPressedAction}
      onclick={() => {
        void runTouchMenuAction('copy', async () => {
          await editor.handleCopy();
        });
      }}
      onpointercancel={clearPressedAction}
      onpointerdown={() => {
        pressedAction = 'copy';
      }}
      onpointerleave={clearPressedAction}
      type="button"
    >
      복사
    </button>
    <div
      class={css({
        width: '1px',
        alignSelf: 'stretch',
        marginY: '4px',
        backgroundColor: 'border.default',
      })}
    ></div>
    <button
      class={css(
        {
          appearance: 'none',
          border: 'none',
          background: 'transparent',
          display: 'inline-flex',
          alignItems: 'center',
          justifyContent: 'center',
          height: '36px',
          paddingX: '14px',
          borderRadius: 'full',
          fontSize: '16px',
          fontWeight: 'medium',
          color: 'text.default',
          whiteSpace: 'nowrap',
          WebkitTapHighlightColor: 'transparent',
          _active: {
            backgroundColor: 'surface.muted',
          },
          _disabled: {
            opacity: '40',
            cursor: 'default',
          },
        },
        pressedAction === 'selectAll' && { backgroundColor: 'surface.muted' },
      )}
      onblur={clearPressedAction}
      onclick={() => {
        void runTouchMenuAction('selectAll', () => {
          editor.handleSelectAll();
        });
      }}
      onpointercancel={clearPressedAction}
      onpointerdown={() => {
        pressedAction = 'selectAll';
      }}
      onpointerleave={clearPressedAction}
      type="button"
    >
      전체 선택
    </button>
  </div>
{/snippet}

{#if isTouchContextMenuOpen && contextMenuPosition}
  {#if contextMenuPlacement.startsWith('top')}
    <div
      style:left={`${contextMenuPosition.x}px`}
      style:top={`${contextMenuPosition.y}px`}
      class={css({
        position: 'fixed',
        width: '0',
        height: '0',
        pointerEvents: 'none',
      })}
      use:topAnchorAction
    ></div>
    <div
      class={css({
        zIndex: 'menu',
        pointerEvents: 'none',
      })}
      data-editor-touch-context-menu
      use:topFloatingAction
    >
      {@render touchContextMenuContent()}
    </div>
  {:else}
    <div
      style:left={`${contextMenuPosition.x}px`}
      style:top={`${contextMenuPosition.y}px`}
      class={css({
        position: 'fixed',
        width: '0',
        height: '0',
        pointerEvents: 'none',
      })}
      use:bottomAnchorAction
    ></div>
    <div
      class={css({
        zIndex: 'menu',
        pointerEvents: 'none',
      })}
      data-editor-touch-context-menu
      use:bottomFloatingAction
    >
      {@render touchContextMenuContent()}
    </div>
  {/if}
{/if}
