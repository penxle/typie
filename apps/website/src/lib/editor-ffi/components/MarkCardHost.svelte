<script lang="ts">
  import { flip, hide, shift } from '@floating-ui/dom';
  import { css } from '@typie/styled-system/css';
  import { createFloatingActions } from '@typie/ui/actions';
  import { pushEscapeHandler } from '@typie/ui/utils';
  import { getEditorContext } from '../editor.svelte';
  import { isSelectionCollapsed, pageRectsToVirtualElement, presentedPageElement, selectionHeadRect } from '../geometry';
  import { normalizeUrl, openLink, openLinkCardAtPoint, removeLinkAtPoint } from '../handlers/link';
  import { linkRectKey, pickLinkTooltipAnchorRect } from './link-tooltip';
  import MarkCard from './MarkCard.svelte';
  import type { PageRect } from '@typie/editor-ffi/browser';
  import type { MarkCardRequest } from '../editor.svelte';

  const ctx = getEditorContext();
  const { editor } = ctx;

  const hover = $derived(editor?.linkHover);

  let activeHover = $state<typeof hover>();
  let isCardHovered = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | null = null;
  let editOverride = $state<{ request: MarkCardRequest; editing: boolean } | null>(null);

  const request = $derived(ctx.markCard);
  const pinnedEditing = $derived(request !== null && (editOverride?.request === request ? editOverride.editing : request.mode === 'edit'));

  const pinnedValue = $derived.by(() => {
    if (!request || !editor) return '';
    const state = editor.modifierState;
    if (request.kind === 'link') return state?.link?.type === 'uniform' ? state.link.value.href : '';
    return state?.ruby?.type === 'uniform' ? state.ruby.value.text : '';
  });

  const hoverTarget = $derived.by(() => {
    if (request || !activeHover || !editor) return;
    const rect = pickLinkTooltipAnchorRect(activeHover.link.rects);
    if (!rect || !presentedPageElement(editor, activeHover.page)) return;
    return { link: activeHover.link, anchor: { page_idx: activeHover.page, rect } satisfies PageRect };
  });

  const canEdit = $derived(!(editor?.readOnly ?? false));
  const canCopy = $derived(!(editor?.protectContent && editor.readOnly));

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom-start',
    offset: 8,
    middleware: [flip(), shift({ padding: 8 }), hide()],
    onClickOutside: () => {
      if (ctx.markCard) close();
    },
  });

  const clearHideTimer = () => {
    if (!hideTimer) return;
    clearTimeout(hideTimer);
    hideTimer = null;
  };

  const close = () => {
    ctx.markCard = null;
  };

  const closeAndFocus = () => {
    close();
    editor?.focus();
  };

  const selectionRects = (): PageRect[] => {
    const snapshot = editor?.published?.snapshot;
    if (!snapshot) return [];
    const endpoints = snapshot.selectionEndpoints;
    if (endpoints && !isSelectionCollapsed(snapshot.selection)) return [endpoints.from, endpoints.to];
    const head = selectionHeadRect(snapshot);
    return head ? [head] : [];
  };

  const centerPoint = (anchorRect: PageRect) => ({
    page: anchorRect.page_idx,
    x: anchorRect.rect.x + anchorRect.rect.width / 2,
    y: anchorRect.rect.y + anchorRect.rect.height / 2,
  });

  const cardKey = $derived(request ? `pinned:${request.kind}` : hoverTarget ? `hover:${linkRectKey(hoverTarget.link)}` : '');

  $effect(() => {
    if (request && !pinnedEditing && !pinnedValue) close();
  });

  $effect(() => {
    clearHideTimer();

    if (hover) {
      activeHover = hover;
      return;
    }

    if (isCardHovered) {
      return;
    }

    hideTimer = setTimeout(() => {
      activeHover = undefined;
      hideTimer = null;
    }, 120);

    return () => clearHideTimer();
  });

  $effect(() => {
    if (!editor) return;
    if (request) {
      const rects = request.anchor ? [request.anchor] : selectionRects();
      if (rects.length > 0) anchor(pageRectsToVirtualElement(editor, rects));
      return;
    }
    if (hoverTarget) anchor(pageRectsToVirtualElement(editor, [hoverTarget.anchor]));
  });

  $effect(() => {
    if (!request) return;
    return editor?.retainFocus();
  });

  $effect(() => {
    if (!request) return;
    return pushEscapeHandler(() => {
      closeAndFocus();
      return true;
    });
  });

  const submitPinned = (value: string) => {
    if (!request || !editor) return;
    if (request.kind === 'link') {
      editor.enqueue({
        type: 'modifier',
        op: { type: 'edit', modifier_type: 'link', modifier: { type: 'link', href: normalizeUrl(value) } },
      });
    } else {
      editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: 'ruby', modifier: { type: 'ruby', text: value } } });
    }
    closeAndFocus();
  };

  const removePinned = () => {
    if (!request || !editor) return;
    editor.enqueue({ type: 'modifier', op: { type: 'edit', modifier_type: request.kind, modifier: undefined } });
    closeAndFocus();
  };

  const noop = () => {
    return;
  };

  const copy = async (href: string) => {
    await navigator.clipboard.writeText(href);
  };
</script>

{#if request && (pinnedEditing || pinnedValue)}
  <div class={css({ zIndex: '50', width: '[fit-content]', maxWidth: '[calc(100vw - 24px)]', pointerEvents: 'auto' })} use:floating>
    {#key cardKey}
      <div
        class={`mark-card ${css({
          borderRadius: '8px',
          backgroundColor: 'surface.default',
          boxShadow: 'lg',
          overflow: 'hidden',
          _dark: { borderWidth: '1px', borderColor: 'border.hairline' },
        })}`}
      >
        <MarkCard
          autofocus
          {canCopy}
          {canEdit}
          editing={pinnedEditing}
          kind={request.kind}
          oncancel={() => {
            if (pinnedValue) editOverride = { request, editing: false };
            else closeAndFocus();
          }}
          oncopy={() => copy(pinnedValue)}
          onedit={() => (editOverride = { request, editing: true })}
          onopen={() => openLink(pinnedValue)}
          onremove={removePinned}
          onsubmit={submitPinned}
          value={pinnedValue}
        />
      </div>
    {/key}
  </div>
{:else if hoverTarget}
  <div
    class={css({ zIndex: '50', width: '[fit-content]', maxWidth: '[calc(100vw - 24px)]', pointerEvents: 'auto' })}
    onpointerenter={() => {
      clearHideTimer();
      isCardHovered = true;
    }}
    onpointerleave={() => {
      isCardHovered = false;
      if (!hover) {
        activeHover = undefined;
      }
    }}
    role="presentation"
    use:floating
  >
    {#key cardKey}
      <div
        class={`mark-card ${css({
          borderRadius: '8px',
          backgroundColor: 'surface.default',
          boxShadow: 'lg',
          overflow: 'hidden',
          _dark: { borderWidth: '1px', borderColor: 'border.hairline' },
        })}`}
      >
        <MarkCard
          {canCopy}
          {canEdit}
          editing={false}
          kind="link"
          oncopy={() => copy(hoverTarget.link.href)}
          onedit={() => {
            const target = hoverTarget;
            activeHover = undefined;
            isCardHovered = false;
            openLinkCardAtPoint({ ctx, editor, point: centerPoint(target.anchor), anchor: target.anchor });
          }}
          onopen={() => openLink(hoverTarget.link.href)}
          onremove={() => {
            const target = hoverTarget;
            activeHover = undefined;
            isCardHovered = false;
            if (editor) removeLinkAtPoint(editor, centerPoint(target.anchor));
          }}
          onsubmit={noop}
          value={hoverTarget.link.href}
        />
      </div>
    {/key}
  </div>
{/if}

<style>
  .mark-card {
    transform-origin: left top;
    animation: mark-card-in 150ms cubic-bezier(0.23, 1, 0.32, 1) both;
  }

  @keyframes mark-card-in {
    from {
      opacity: 0;
      transform: scale(0.96);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .mark-card {
      animation: none;
    }
  }
</style>
