<script lang="ts">
  import { hide, inline, shift, size } from '@floating-ui/dom';
  import { flex } from '@typie/styled-system/patterns';
  import { createFloatingActions } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import ArrowRightIcon from '~icons/lucide/arrow-right';
  import XIcon from '~icons/lucide/x';
  import type { Editor } from '$lib/editor/editor.svelte';

  type Props = {
    editor: Editor;
  };

  let { editor }: Props = $props();

  const activeError = $derived(editor.spellcheckErrors.find((v) => v.active));
  const activeItem = $derived(editor.trackedItems.find((v) => v.id === activeError?.id));

  const scroller = $derived.by(() => editor.scrollContainerEl);

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 4,
    middleware: [
      inline(),
      shift({
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 8,
      }),
      size({
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 8,
        apply({ availableWidth, elements }) {
          Object.assign(elements.floating.style, {
            maxWidth: `${availableWidth}px`,
          });
        },
      }),
      hide({
        strategy: 'escaped',
        get boundary() {
          return scroller ?? undefined;
        },
        padding: 32,
      }),
    ],
  });

  $effect(() => {
    if (activeItem?.bounds?.[0]) {
      const pageIdx = activeItem.pageIdx;

      const virtualEl = {
        getBoundingClientRect: () => {
          const pageEl = editor.pageContainerEls[pageIdx];
          if (!pageEl || activeItem.bounds.length === 0) return new DOMRect();

          const pageRect = pageEl.getBoundingClientRect();
          const rects = activeItem.bounds.map((b) => new DOMRect(pageRect.left + b.x, pageRect.top + b.y, b.width, b.height));

          let minX = Infinity;
          let minY = Infinity;
          let maxX = -Infinity;
          let maxY = -Infinity;

          for (const r of rects) {
            minX = Math.min(minX, r.left);
            minY = Math.min(minY, r.top);
            maxX = Math.max(maxX, r.right);
            maxY = Math.max(maxY, r.bottom);
          }

          return new DOMRect(minX, minY, maxX - minX, maxY - minY);
        },
        getClientRects: () => {
          const pageEl = editor.pageContainerEls[pageIdx];
          if (!pageEl) return [];

          const pageRect = pageEl.getBoundingClientRect();
          return activeItem.bounds.map((b) => new DOMRect(pageRect.left + b.x, pageRect.top + b.y, b.width, b.height));
        },
      };
      anchor(virtualEl);
    }
  });

  const applyCorrection = (correction: string) => {
    if (!activeItem) return;

    const success = editor.replaceTextInBlock(activeItem.nodeId, activeItem.startOffset, activeItem.endOffset, correction);
    if (success) {
      editor.removeTrackedItems(0, [activeItem.id]);
    }

    editor.focus();
  };

  const removeError = () => {
    if (!activeError) return;

    editor.removeTrackedItems(0, [activeError.id]);
    editor.focus();
  };
</script>

{#if activeError && activeItem}
  <div
    class={flex({
      alignItems: 'center',
      gap: '4px',
      zIndex: 'overEditor',
      wrap: 'wrap',
      pointerEvents: 'auto',
    })}
    use:floating={{ appendTo: scroller }}
  >
    {#if activeError.corrections.length > 0}
      {#each activeError.corrections as correction (correction)}
        <button
          class={flex({
            justifyContent: 'space-between',
            alignItems: 'center',
            gap: '4px',
            borderWidth: '1px',
            borderColor: 'border.danger',
            borderRadius: '4px',
            paddingX: '8px',
            paddingY: '4px',
            fontSize: '13px',
            fontWeight: 'semibold',
            color: 'text.danger',
            backgroundColor: 'accent.danger.subtle',
            transition: 'common',
            boxShadow: 'small',
            _hover: {
              backgroundColor: { base: 'red.100', _dark: 'dark.red.800' },
            },
          })}
          onclick={() => applyCorrection(correction)}
          type="button"
        >
          {correction}
          <Icon icon={ArrowRightIcon} size={12} />
        </button>
      {/each}
    {/if}

    <button
      class={flex({
        alignItems: 'center',
        justifyContent: 'center',
        size: '29px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '4px',
        backgroundColor: 'surface.default',
        color: 'text.faint',
        transition: 'common',
        boxShadow: 'small',
        _hover: {
          backgroundColor: 'surface.muted',
          color: 'text.subtle',
        },
      })}
      onclick={removeError}
      type="button"
    >
      <Icon icon={XIcon} size={14} />
    </button>
  </div>
{/if}
