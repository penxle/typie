<script lang="ts">
  import { flip, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import { TAP_FEEDBACK_MIN_MS, TOUCH_MENU_GAP, TOUCH_MENU_VIEWPORT_PADDING } from '$lib/editor-ffi/constants';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';

  const ctx = getEditorContext();

  let pressedAction = $state<'copy' | 'selectAll' | null>(null);

  const isTouchContextMenuOpen = $derived(!!ctx.editor && ctx.editor.contextMenu.isOpen && ctx.editor.contextMenu.source === 'touch');
  const contextMenuPosition = $derived(
    isTouchContextMenuOpen && ctx.editor ? { x: ctx.editor.contextMenu.x, y: ctx.editor.contextMenu.y } : null,
  );
  const contextMenuPlacement = $derived(ctx.editor?.contextMenu.placement ?? 'bottom');

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
    ctx.editor?.closeContextMenu();
    ctx.editor?.focus();
  };

  const clearPressedAction = () => {
    pressedAction = null;
  };

  const waitForTapFeedback = () => new Promise((resolve) => setTimeout(resolve, TAP_FEEDBACK_MIN_MS));

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
    if (!isTouchContextMenuOpen) return;
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest('[data-editor-touch-context-menu]')) return;
    clearPressedAction();
    ctx.editor?.closeContextMenu();
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
          _active: { backgroundColor: 'surface.muted' },
          _disabled: { opacity: '40', cursor: 'default' },
        },
        pressedAction === 'copy' && { backgroundColor: 'surface.muted' },
      )}
      disabled={ctx.editor?.isSelectionCollapsed ?? true}
      onblur={clearPressedAction}
      onclick={(e) => {
        e.stopPropagation();
        void runTouchMenuAction('copy', async () => {
          await ctx.editor?.requestCopy();
        });
      }}
      onpointercancel={clearPressedAction}
      onpointerdown={(e) => {
        e.stopPropagation();
        pressedAction = 'copy';
      }}
      onpointerleave={clearPressedAction}
      type="button"
    >
      복사
    </button>
    <div class={css({ width: '1px', alignSelf: 'stretch', marginY: '4px', backgroundColor: 'border.default' })}></div>
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
          _active: { backgroundColor: 'surface.muted' },
          _disabled: { opacity: '40', cursor: 'default' },
        },
        pressedAction === 'selectAll' && { backgroundColor: 'surface.muted' },
      )}
      onblur={clearPressedAction}
      onclick={(e) => {
        e.stopPropagation();
        void runTouchMenuAction('selectAll', () => {
          ctx.editor?.requestSelectAll();
        });
      }}
      onpointercancel={clearPressedAction}
      onpointerdown={(e) => {
        e.stopPropagation();
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
  {#if contextMenuPlacement === 'top'}
    <div
      style:left={`${contextMenuPosition.x}px`}
      style:top={`${contextMenuPosition.y}px`}
      class={css({ position: 'fixed', width: '0', height: '0', pointerEvents: 'none' })}
      use:topAnchorAction
    ></div>
    <div class={css({ zIndex: 'menu', pointerEvents: 'none' })} data-editor-touch-context-menu use:topFloatingAction>
      {@render touchContextMenuContent()}
    </div>
  {:else}
    <div
      style:left={`${contextMenuPosition.x}px`}
      style:top={`${contextMenuPosition.y}px`}
      class={css({ position: 'fixed', width: '0', height: '0', pointerEvents: 'none' })}
      use:bottomAnchorAction
    ></div>
    <div class={css({ zIndex: 'menu', pointerEvents: 'none' })} data-editor-touch-context-menu use:bottomFloatingAction>
      {@render touchContextMenuContent()}
    </div>
  {/if}
{/if}
