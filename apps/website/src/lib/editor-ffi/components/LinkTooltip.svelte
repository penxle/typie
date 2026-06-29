<script lang="ts">
  import { flip, hide, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import CopyIcon from '~icons/lucide/copy';
  import GlobeIcon from '~icons/lucide/globe';
  import { getEditorContext } from '../editor.svelte';
  import { pageRectsToVirtualElement } from '../geometry';
  import { openLinkEditorFromTooltip } from '../handlers/link';
  import { linkRectKey, pickLinkTooltipAnchorRect, resolveSelectionTarget } from './link-tooltip';

  const ctx = getEditorContext();
  const { editor } = ctx;

  const hover = $derived(editor?.linkHover);

  type TooltipTarget = import('./link-tooltip').LinkTooltipTarget;

  let activeHover = $state<typeof hover>();
  let isTooltipHovered = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;

  const keyboardTarget = $derived.by<TooltipTarget | undefined>(() => {
    const instance = editor;
    if (!instance) return;
    // Read the whole-document `linkRects` (O(pages · N)) only when the caret is
    // actually inside a uniform link — otherwise `resolveSelectionTarget` returns
    // immediately, so guarding here keeps that cost off the normal typing path.
    const modifierStateLink = instance.modifierState?.link;
    if (modifierStateLink?.type !== 'uniform') return;
    return resolveSelectionTarget({
      linkRects: instance.linkRects,
      modifierStateLink,
      selection: instance.selection,
      selectionHeadRect: instance.selectionHeadRect(),
    });
  });

  const tooltipTarget = $derived.by<TooltipTarget | undefined>(() => {
    if (activeHover) {
      // Anchor to the link's own first rect, independent of the pointer, so the
      // tooltip stays fixed while the mouse moves within the link.
      const anchorRect = pickLinkTooltipAnchorRect(activeHover.link.rects);
      if (anchorRect) {
        return {
          link: activeHover.link,
          page: activeHover.page,
          anchorRect,
        };
      }
    }

    return keyboardTarget;
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 8,
    middleware: [flip(), shift({ padding: 8 }), hide()],
  });

  const clearHideTimer = () => {
    if (!hideTimer) {
      return;
    }

    clearTimeout(hideTimer);
    hideTimer = null;
  };

  const copyHref = async () => {
    if (editor?.readOnly && editor.protectContent) return;
    const href = tooltipTarget?.link.href;
    if (!href) return;
    await navigator.clipboard.writeText(href);
  };

  const openToolbarLinkEditor = async (target: TooltipTarget) => {
    const rect = target.anchorRect;
    await openLinkEditorFromTooltip({
      closeTooltip: () => {
        activeHover = undefined;
        isTooltipHovered = false;
      },
      ctx,
      editor,
      point: { page: target.page, x: rect.x + rect.width / 2, y: rect.y + rect.height / 2 },
    });
  };

  $effect(() => {
    clearHideTimer();

    if (hover) {
      const nodeChanged = !activeHover || linkRectKey(activeHover.link) !== linkRectKey(hover.link);
      activeHover = hover;
      if (nodeChanged) {
        ctx.linkEditorOpen = false;
      }
      return;
    }

    if (isTooltipHovered) {
      return;
    }

    hideTimer = setTimeout(() => {
      activeHover = undefined;
      hideTimer = null;
    }, 120);

    return () => clearHideTimer();
  });

  // Anchor the tooltip to the link rect via a floating-ui virtual element built
  // from page-local rects. autoUpdate re-reads it on scroll/resize, keeping the
  // tooltip pinned to the link. (Same pattern as the spellcheck/comment popovers.)
  $effect(() => {
    const target = tooltipTarget;
    if (target && editor) {
      anchor(pageRectsToVirtualElement(editor, [{ page_idx: target.page, rect: target.anchorRect }]));
    }
  });
</script>

{#if tooltipTarget}
  <div
    class={css({
      zIndex: '50',
      width: '[fit-content]',
      maxWidth: '[min(320px, calc(100vw - 24px))]',
      pointerEvents: 'auto',
    })}
    onpointerenter={() => {
      clearHideTimer();
      isTooltipHovered = true;
    }}
    onpointerleave={() => {
      isTooltipHovered = false;
      if (!hover) {
        activeHover = undefined;
      }
    }}
    role="presentation"
    use:floating
  >
    <div
      class={css({
        borderWidth: '1px',
        borderColor: 'border.subtle',
        borderRadius: '4px',
        paddingX: '4px',
        paddingY: '4px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        overflow: 'hidden',
      })}
    >
      <div
        class={css({
          display: 'flex',
          alignItems: 'center',
          gap: '4px',
          minWidth: '0',
          borderRadius: '4px',
          minHeight: '24px',
          paddingLeft: '8px',
          paddingRight: '4px',
          paddingY: '2px',
          backgroundColor: 'surface.default',
          color: 'text.subtle',
        })}
      >
        <Icon style={css.raw({ color: 'var(--colors-text-faint)' })} icon={GlobeIcon} size={12} />
        <span
          class={css({
            display: 'block',
            minWidth: '0',
            flex: '1',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
            whiteSpace: 'nowrap',
            fontSize: '12px',
            lineHeight: '[1.2]',
            color: 'text.subtle',
          })}
        >
          {tooltipTarget.link.href}
        </span>

        {#if !(editor?.readOnly && editor.protectContent)}
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: '0',
              width: '20px',
              height: '20px',
              borderRadius: '4px',
              color: 'text.subtle',
              transition: 'common',
            })}
            aria-label="링크 복사"
            onclick={copyHref}
            type="button"
          >
            <Icon icon={CopyIcon} size={12} />
          </button>
        {/if}

        {#if !(editor?.readOnly ?? false)}
          <button
            class={css({
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              flexShrink: '0',
              height: '20px',
              borderRadius: '4px',
              paddingX: '6px',
              fontSize: '11px',
              fontWeight: 'medium',
              color: 'text.subtle',
              transition: 'common',
            })}
            onclick={() => openToolbarLinkEditor(tooltipTarget)}
            type="button"
          >
            편집
          </button>
        {/if}
      </div>
    </div>
  </div>
{/if}
