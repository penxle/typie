<script lang="ts">
  import { flip, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { scale } from 'svelte/transition';
  import { TAP_FEEDBACK_MIN_MS, TOUCH_MENU_GAP, TOUCH_MENU_VIEWPORT_PADDING } from '$lib/editor-ffi/constants';
  import { getEditorContext } from '$lib/editor-ffi/editor.svelte';
  import { getContextMenuCapabilityState } from './context-menu-state';

  const ctx = getEditorContext();

  let pressedAction = $state<string | null>(null);

  const isTouchContextMenuOpen = $derived(!!ctx.editor && ctx.editor.contextMenu.isOpen && ctx.editor.contextMenu.source === 'touch');
  const contextMenuPosition = $derived(
    isTouchContextMenuOpen && ctx.editor ? { x: ctx.editor.contextMenu.x, y: ctx.editor.contextMenu.y } : null,
  );
  const contextMenuPlacement = $derived(ctx.editor?.contextMenu.placement ?? 'bottom');
  const extraItems = $derived(ctx.editor?.contextMenu.extraItems ?? []);
  const capabilityState = $derived(
    getContextMenuCapabilityState({
      isSelectionCollapsed: ctx.editor?.isSelectionCollapsed ?? true,
      readOnly: ctx.editor?.readOnly ?? false,
      protectContent: ctx.editor?.protectContent ?? false,
    }),
  );

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

  const runTouchMenuAction = async (action: string, fn: () => void | Promise<void>) => {
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

  const buttonStyle = (action: string) =>
    css(
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
      pressedAction === action && { backgroundColor: 'surface.muted' },
    );

  const dividerStyle = css({ width: '1px', alignSelf: 'stretch', marginY: '4px', backgroundColor: 'border.default' });
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
      class={buttonStyle('copy')}
      disabled={capabilityState.copyDisabled}
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
    <div class={dividerStyle}></div>
    <button
      class={buttonStyle('cut')}
      disabled={capabilityState.cutDisabled}
      onblur={clearPressedAction}
      onclick={(e) => {
        e.stopPropagation();
        void runTouchMenuAction('cut', async () => {
          await ctx.editor?.requestCut();
        });
      }}
      onpointercancel={clearPressedAction}
      onpointerdown={(e) => {
        e.stopPropagation();
        pressedAction = 'cut';
      }}
      onpointerleave={clearPressedAction}
      type="button"
    >
      잘라내기
    </button>
    <div class={dividerStyle}></div>
    <button
      class={buttonStyle('paste')}
      disabled={capabilityState.pasteDisabled}
      onblur={clearPressedAction}
      onclick={(e) => {
        e.stopPropagation();
        void runTouchMenuAction('paste', async () => {
          await ctx.editor?.requestPaste();
        });
      }}
      onpointercancel={clearPressedAction}
      onpointerdown={(e) => {
        e.stopPropagation();
        pressedAction = 'paste';
      }}
      onpointerleave={clearPressedAction}
      type="button"
    >
      붙여넣기
    </button>
    <div class={dividerStyle}></div>
    <button
      class={buttonStyle('selectAll')}
      disabled={capabilityState.selectAllDisabled}
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
    {#each extraItems as item, i (i)}
      <div class={dividerStyle}></div>
      <button
        class={buttonStyle(`extra-${i}`)}
        onblur={clearPressedAction}
        onclick={(e) => {
          e.stopPropagation();
          void runTouchMenuAction(`extra-${i}`, async () => {
            await item.onclick();
          });
        }}
        onpointercancel={clearPressedAction}
        onpointerdown={(e) => {
          e.stopPropagation();
          pressedAction = `extra-${i}`;
        }}
        onpointerleave={clearPressedAction}
        type="button"
      >
        {item.label}
      </button>
    {/each}
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
